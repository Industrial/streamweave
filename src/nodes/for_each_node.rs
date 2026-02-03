//! # ForEach Node
//!
//! A transform node that expands collections into streams by emitting each collection item as a separate output.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define how to extract collections
//! - **Input**: `"in"` - Receives data items containing collections
//! - **Output**: `"out"` - Sends each item from the collections
//! - **Output**: `"error"` - Sends errors that occur during collection extraction or iteration
//!
//! ## Configuration
//!
//! The configuration port receives `ForEachConfig` (which is `Arc<dyn ForEachFunction>`) that defines
//! how to extract a collection from input items. The node iterates over each collection and emits
//! each item as a separate output.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous collection extraction functions used by ForEachNode.
///
/// Implementations of this trait define how to extract a collection from input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns an iterator over collection items.
#[async_trait]
pub trait ForEachFunction: Send + Sync {
  /// Extracts a collection from the input value and returns an iterator over its items.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(items)` where `items` is a vector of items to emit, or `Err(error_message)` if extraction fails.
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String>;
}

/// Configuration for ForEachNode that defines the collection extraction function.
///
/// Contains an Arc-wrapped function that implements `ForEachFunction` to perform the extraction.
pub type ForEachConfig = Arc<dyn ForEachFunction>;

/// Wrapper type that implements ForEachFunction for async closures.
struct ForEachFunctionWrapper<F> {
  function: F,
}

#[async_trait]
impl<F> ForEachFunction for ForEachFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
    ) -> std::pin::Pin<
      Box<dyn std::future::Future<Output = Result<Vec<Arc<dyn Any + Send + Sync>>, String>> + Send>,
    > + Send
    + Sync,
{
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a ForEachConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::{ForEachConfig, for_each_config};
///
/// // Create a config that extracts a Vec<i32> and emits each item
/// let config: ForEachConfig = for_each_config(|value| async move {
///     if let Ok(arc_vec) = value.downcast::<Vec<i32>>() {
///         let items: Vec<Arc<dyn Any + Send + Sync>> = arc_vec.iter()
///             .map(|&n| Arc::new(n) as Arc<dyn Any + Send + Sync>)
///             .collect();
///         Ok(items)
///     } else {
///         Err("Expected Vec<i32>".to_string())
///     }
/// });
/// ```
pub fn for_each_config<F, Fut>(function: F) -> ForEachConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut:
    std::future::Future<Output = Result<Vec<Arc<dyn Any + Send + Sync>>, String>> + Send + 'static,
{
  Arc::new(ForEachFunctionWrapper {
    function: move |v| {
      Box::pin(function(v))
        as std::pin::Pin<
          Box<
            dyn std::future::Future<Output = Result<Vec<Arc<dyn Any + Send + Sync>>, String>>
              + Send,
          >,
        >
    },
  })
}

/// A node that expands collections into streams by emitting each collection item.
///
/// The node receives configuration that defines how to extract collections from input items,
/// and emits each item from the collection as a separate output.
pub struct ForEachNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current configuration state.
  current_config: Arc<Mutex<Option<ForEachConfig>>>,
}

impl ForEachNode {
  /// Creates a new ForEachNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::ForEachNode;
  ///
  /// let node = ForEachNode::new("for_each".to_string());
  /// // Creates ports: configuration, in → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
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
impl Node for ForEachNode {
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

      // Tag streams to distinguish config from data
      let config_stream = config_stream.map(|item| (MessageType::Config, item));
      let data_stream = data_stream.map(|item| (MessageType::Data, item));

      // Merge streams
      let merged_stream = stream::select(config_stream, data_stream);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut current_config: Option<ForEachConfig> = None;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration - handle both Arc<Arc<dyn ForEachFunction>> and Arc<dyn ForEachFunction>
              if let Ok(arc_arc_fn) = item.clone().downcast::<Arc<Arc<dyn ForEachFunction>>>() {
                let cfg = Arc::clone(&**arc_arc_fn);
                current_config = Some(Arc::clone(&cfg));
                *config_state_clone.lock().await = Some(cfg);
              } else if let Ok(arc_function) = item.downcast::<Arc<dyn ForEachFunction>>() {
                current_config = Some(Arc::clone(&*arc_function));
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_function));
              } else {
                let error_msg: String =
                  "Invalid configuration type - expected ForEachConfig (Arc<dyn ForEachFunction>)"
                    .to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              match &current_config {
                Some(cfg) => {
                  // Zero-copy: cfg.apply receives item by value (Arc), only refcount is incremented
                  match cfg.apply(item).await {
                    Ok(items) => {
                      // Emit each item from the collection (zero-copy: only Arc refcount increments)
                      for item in items {
                        let _ = out_tx_clone.send(item).await;
                      }
                    }
                    Err(error_msg) => {
                      let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                      let _ = error_tx_clone.send(error_arc).await;
                    }
                  }
                }
                None => {
                  let error_msg: String =
                    "No configuration set. Please send configuration before data.".to_string();
                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
          }
        }
      });

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
