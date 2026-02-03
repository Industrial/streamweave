//! # Filter Node
//!
//! A transform node that filters data items based on a configurable predicate function.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define the filter predicate
//! - **Input**: `"in"` - Receives data items to filter
//! - **Output**: `"out"` - Sends data items that pass the filter
//! - **Output**: `"error"` - Sends errors that occur during filtering
//!
//! ## Configuration
//!
//! The configuration port receives `FilterConfig` (which is `Arc<dyn FilterFunction>`) that defines
//! the predicate function. The node applies the configured predicate to each item received on the
//! "in" port and only passes through items where the predicate returns `true`.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, process_configurable_node};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

/// Trait for asynchronous filter predicate functions used by FilterNode.
///
/// Implementations of this trait define how to filter input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns
/// either `Ok(true)` to pass the item through, `Ok(false)` to filter it out,
/// or `Err(error_message)` if an error occurs.
#[async_trait]
pub trait FilterFunction: Send + Sync {
  /// Applies the filter predicate to the input value.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(true)` if the item should pass through, `Ok(false)` if it should be filtered out,
  /// or `Err(error_message)` if an error occurs.
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<bool, String>;
}

/// Configuration for FilterNode that defines the filter predicate.
///
/// Uses Arc for zero-copy sharing.
pub type FilterConfig = Arc<dyn FilterFunction>;

/// Wrapper type that implements FilterFunction for async closures.
struct FilterFunctionWrapper<F> {
  function: F,
}

#[async_trait]
impl<F> FilterFunction for FilterFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send>>
    + Send
    + Sync,
{
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<bool, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a FilterConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::{FilterConfig, filter_config};
///
/// // Create a config that filters out negative numbers
/// let config: FilterConfig = filter_config(|value| async move {
///     if let Ok(arc_i32) = value.downcast::<i32>() {
///         Ok(*arc_i32 >= 0)
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn filter_config<F, Fut>(function: F) -> FilterConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<bool, String>> + Send + 'static,
{
  Arc::new(FilterFunctionWrapper {
    function: move |v| {
      Box::pin(function(v))
        as std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send>>
    },
  })
}

/// A node that filters data items based on a configurable predicate function.
///
/// The filter predicate is defined by configuration received on the "configuration" port.
/// Each item from the "in" port is tested against the current predicate. Items where the
/// predicate returns `true` are sent to the "out" port. Items where it returns `false` are
/// filtered out (dropped). Errors are sent to the "error" port.
pub struct FilterNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current configuration state.
  current_config: Arc<Mutex<Option<Arc<FilterConfig>>>>,
}

impl FilterNode {
  /// Creates a new FilterNode with the given name.
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
      current_config: Arc::new(Mutex::new(None::<Arc<FilterConfig>>)),
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
impl Node for FilterNode {
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
      // FilterNode needs special handling: it passes through the original item if filter returns true
      let (out_rx, error_rx) = process_configurable_node(
        config_stream,
        data_stream,
        config_state,
        |item: Arc<dyn Any + Send + Sync>, cfg: &Arc<FilterConfig>| {
          let cfg = cfg.clone();
          async move {
            // Zero-copy: clone Arc reference before calling apply (atomic refcount increment, ~1-2ns)
            // This allows us to pass the item through if filter returns true
            let item_clone = item.clone();
            match cfg.apply(item).await {
              Ok(true) => {
                // Item passes filter - return it (zero-copy: only Arc refcount was incremented)
                Ok(Some(item_clone))
              }
              Ok(false) => {
                // Item filtered out - return None (zero-copy: item is dropped, no data copy occurred)
                Ok(None)
              }
              Err(e) => Err(e),
            }
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
