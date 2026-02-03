//! # Map Node
//!
//! A transform node that applies a configurable transformation to each input item.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define the transformation
//! - **Input**: `"in"` - Receives data items to transform
//! - **Output**: `"out"` - Sends transformed data items
//! - **Output**: `"error"` - Sends errors that occur during transformation
//!
//! ## Configuration
//!
//! The configuration port receives `MapConfig` (which is `Arc<dyn MapFunction>`) that defines
//! what transformation to apply. The node applies the configured transformation to each
//! item received on the "in" port.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, process_configurable_node};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

/// Trait for asynchronous transformation functions used by MapNode.
///
/// Implementations of this trait define how to transform input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns
/// either a transformed value or an error message.
#[async_trait]
pub trait MapFunction: Send + Sync {
  /// Applies the transformation to the input value.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(transformed)` if transformation succeeds, or `Err(error_message)` if it fails.
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String>;
}

/// Configuration for MapNode that defines the transformation operation.
///
/// Uses the unified NodeConfig type for consistency across all nodes.
/// MapConfig is a type alias for convenience, but internally uses NodeConfig.
pub type MapConfig = Arc<dyn MapFunction>;

/// Wrapper type that implements MapFunction for async closures.
struct MapFunctionWrapper<F> {
  /// The async function to wrap.
  function: F,
}

#[async_trait]
impl<F> MapFunction for MapFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
    ) -> std::pin::Pin<
      Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>,
    > + Send
    + Sync,
{
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a MapConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::{MapConfig, map_config};
///
/// // Create a config that multiplies by 2
/// let config: MapConfig = map_config(|value| async move {
///     if let Ok(arc_i32) = value.downcast::<i32>() {
///         Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn map_config<F, Fut>(function: F) -> MapConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send + 'static,
{
  Arc::new(MapFunctionWrapper {
    function: move |v| {
      Box::pin(function(v))
        as std::pin::Pin<
          Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>,
        >
    },
  })
}

/// A node that applies a configurable transformation to each input item.
///
/// The transformation is defined by configuration received on the "configuration" port.
/// Each item from the "in" port is transformed according to the current configuration
/// and sent to the "out" port. Errors are sent to the "error" port.
pub struct MapNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current map function configuration state.
  current_config: Arc<Mutex<Option<Arc<MapConfig>>>>,
}

impl MapNode {
  /// Creates a new MapNode with the given name.
  ///
  /// The node starts with no configuration and will wait for configuration
  /// before processing data.
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None::<Arc<MapConfig>>)),
    }
  }

  /// Returns whether the node has a configuration set.
  pub fn has_config(&self) -> bool {
    self
      .current_config
      .try_lock()
      .map(|g| g.is_some())
      .unwrap_or(false)
  }
}

#[async_trait]
impl Node for MapNode {
  fn name(&self) -> &str {
    self.base.name()
  }

  fn set_name(&mut self, name: &str) {
    self.base.set_name(name);
  }

  fn input_port_names(&self) -> &[String] {
    self.base.input_port_names()
  }

  fn output_port_names(&self) -> &[String] {
    self.base.output_port_names()
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.base.has_input_port(name)
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.base.has_output_port(name)
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let config_state = Arc::clone(&self.current_config);

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;
      let data_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      // Process using unified helper with zero-copy guarantee
      let (out_rx, error_rx) = process_configurable_node(
        config_stream,
        data_stream,
        config_state,
        |item: Arc<dyn Any + Send + Sync>, cfg: &Arc<MapConfig>| {
          let cfg = cfg.clone();
          async move {
            // Zero-copy: cfg.apply receives item by value (Arc), only refcount is incremented
            cfg.apply(item).await.map(Some)
          }
        },
      );

      // Convert channels to streams
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(error_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );

      Ok(outputs)
    })
  }
}
