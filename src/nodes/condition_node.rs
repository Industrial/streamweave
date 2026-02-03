//! # Condition Node
//!
//! A transform node that routes data items to different outputs based on a configurable condition function.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define the condition predicate
//! - **Input**: `"in"` - Receives data items to evaluate
//! - **Output**: `"true"` - Sends data items where the condition evaluates to `true`
//! - **Output**: `"false"` - Sends data items where the condition evaluates to `false`
//! - **Output**: `"error"` - Sends errors that occur during condition evaluation
//!
//! ## Configuration
//!
//! The configuration port receives `ConditionConfig` (which is `Arc<dyn ConditionFunction>`) that defines
//! the condition predicate. The node applies the configured predicate to each item received on the
//! "in" port and routes items to the "true" or "false" output port based on the result.

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

/// Trait for asynchronous condition predicate functions used by ConditionNode.
///
/// Implementations of this trait define how to evaluate conditions on input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns
/// either `Ok(true)` to route to the "true" port, `Ok(false)` to route to the "false" port,
/// or `Err(error_message)` if an error occurs.
#[async_trait]
pub trait ConditionFunction: Send + Sync {
  /// Evaluates the condition predicate on the input value.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(true)` if the item should be routed to the "true" port, `Ok(false)` if it should be
  /// routed to the "false" port, or `Err(error_message)` if an error occurs.
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<bool, String>;
}

/// Configuration for ConditionNode that defines the condition predicate.
///
/// Contains an Arc-wrapped function that implements `ConditionFunction` to perform the evaluation.
/// Using Arc allows the config to be shared and cloned while still being Send + Sync.
pub type ConditionConfig = Arc<dyn ConditionFunction>;

/// Wrapper type that implements ConditionFunction for async closures.
struct ConditionFunctionWrapper<F> {
  function: F,
}

#[async_trait::async_trait]
impl<F> ConditionFunction for ConditionFunctionWrapper<F>
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

/// Helper function to create a ConditionConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::{ConditionConfig, condition_config};
///
/// // Create a config that routes positive numbers to "true" and negative to "false"
/// let config: ConditionConfig = condition_config(|value| async move {
///     if let Ok(arc_i32) = value.downcast::<i32>() {
///         Ok(*arc_i32 >= 0)
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn condition_config<F, Fut>(function: F) -> ConditionConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<bool, String>> + Send + 'static,
{
  Arc::new(ConditionFunctionWrapper {
    function: move |v| {
      Box::pin(function(v))
        as std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send>>
    },
  })
}

/// A node that routes data items to different outputs based on a configurable condition function.
///
/// The condition predicate is defined by configuration received on the "configuration" port.
/// Each item from the "in" port is evaluated against the current predicate. Items where the
/// predicate returns `true` are sent to the "true" port. Items where it returns `false` are
/// sent to the "false" port. Errors are sent to the "error" port.
pub struct ConditionNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<dyn ConditionFunction>>>>,
}

impl ConditionNode {
  /// Creates a new ConditionNode with the given name.
  ///
  /// The node starts with no configuration and will wait for configuration
  /// before processing data.
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["true".to_string(), "false".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None::<Arc<dyn ConditionFunction>>)),
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
impl Node for ConditionNode {
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

      // Create output streams
      let (true_tx, true_rx) = tokio::sync::mpsc::channel(10);
      let (false_tx, false_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let true_tx_clone = true_tx.clone();
      let false_tx_clone = false_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut current_config: Option<Arc<dyn ConditionFunction>> = None;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration
              if let Ok(arc_arc_fn) = item.clone().downcast::<Arc<Arc<dyn ConditionFunction>>>() {
                current_config = Some((**arc_arc_fn).clone());
                *config_state_clone.lock().await = Some((**arc_arc_fn).clone());
              } else if let Ok(arc_function) = item.clone().downcast::<Arc<dyn ConditionFunction>>()
              {
                current_config = Some(Arc::clone(&*arc_function));
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_function));
              } else {
                let error_msg: String = "Invalid configuration type".to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              match &current_config {
                Some(cfg) => {
                  // Zero-copy: clone Arc reference before calling apply (atomic refcount increment, ~1-2ns)
                  // This allows us to route the item to the appropriate port
                  let item_clone = item.clone();
                  match cfg.apply(item).await {
                    Ok(true) => {
                      // Condition is true - route to "true" port (zero-copy: only Arc refcount was incremented)
                      let _ = true_tx_clone.send(item_clone).await;
                    }
                    Ok(false) => {
                      // Condition is false - route to "false" port (zero-copy: only Arc refcount was incremented)
                      // We need another clone for the false port since item_clone was moved above
                      let item_clone2 = item_clone.clone();
                      let _ = false_tx_clone.send(item_clone2).await;
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
        "true".to_string(),
        Box::pin(ReceiverStream::new(true_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "false".to_string(),
        Box::pin(ReceiverStream::new(false_rx))
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
