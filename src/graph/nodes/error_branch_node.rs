//! # ErrorBranch Node
//!
//! A transform node that routes `Result<T, E>` values to different output ports based on whether
//! they are `Ok` or `Err`.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates (optional, for future extensibility)
//! - **Input**: `"in"` - Receives `Result<T, E>` values to route
//! - **Output**: `"success"` - Sends `Ok` values
//! - **Output**: `"error"` - Sends `Err` values
//!
//! ## Behavior
//!
//! The node receives items that are expected to be `Result<T, E>` types. It routes:
//! - `Ok(value)` → `success` port
//! - `Err(error)` → `error` port
//!
//! If an item is not a `Result` type, it is sent to the `error` port with an appropriate error message.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Configuration for ErrorBranchNode (currently unused, reserved for future extensibility).
pub struct ErrorBranchConfig {
  // Reserved for future configuration options
}

/// A node that routes `Result<T, E>` values to success or error output ports.
///
/// The node receives items that should be `Result` types and routes them based on
/// whether they are `Ok` or `Err`.
pub struct ErrorBranchNode {
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<ErrorBranchConfig>>>>,
}

impl ErrorBranchNode {
  /// Creates a new ErrorBranchNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::ErrorBranchNode;
  ///
  /// let node = ErrorBranchNode::new("error_branch".to_string());
  /// // Creates ports: configuration, in → success, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["success".to_string(), "error".to_string()],
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
impl Node for ErrorBranchNode {
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
      let (success_tx, success_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let success_tx_clone = success_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration (currently unused, but store for future use)
              if let Ok(arc_config) = item.clone().downcast::<Arc<ErrorBranchConfig>>() {
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_config));
              } else {
                // Configuration is optional, ignore invalid types
              }
            }
            MessageType::Data => {
              // Try to downcast to Result types
              // We need to handle Result<T, E> where T and E can be various types
              // Since we can't know the exact types at compile time, we'll try common patterns

              // Try Result<Arc<dyn Any + Send + Sync>, String> (generic result)
              // This is the most flexible pattern for type-erased results
              // Users should wrap their Result values in this format
              if let Ok(arc_result) = item
                .clone()
                .downcast::<Result<Arc<dyn Any + Send + Sync>, String>>()
              {
                // Match on reference to avoid moving out of Arc
                match &*arc_result {
                  Ok(value) => {
                    let _ = success_tx_clone.send(value.clone()).await;
                  }
                  Err(error) => {
                    let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error.clone());
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
                continue;
              }

              // If we can't downcast to a known Result type, treat as error
              let error_msg = format!(
                "Expected Result type, but received {}",
                std::any::type_name_of_val(&*item)
              );
              let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
              let _ = error_tx_clone.send(error_arc).await;
            }
          }
        }
      });

      // Convert channels to streams
      let mut outputs = HashMap::new();
      outputs.insert(
        "success".to_string(),
        Box::pin(ReceiverStream::new(success_rx))
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
