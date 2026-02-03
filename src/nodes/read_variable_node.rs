//! # ReadVariable Node
//!
//! A convenience node that reads variable values from a shared VariableNode store.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives the variable store reference (Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>>)
//! - **Input**: `"name"` - Receives variable names (String) to read
//! - **Output**: `"out"` - Sends variable values
//! - **Output**: `"error"` - Sends errors that occur during read operations
//!
//! ## Behavior
//!
//! The node reads variable values from a shared variable store. The store reference must be
//! provided via the configuration port. For each variable name received on the "name" port,
//! the node looks up the variable in the store and sends its value to the "out" port.
//! If the variable doesn't exist, an error is sent to the "error" port.

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

/// Configuration for ReadVariableNode - contains the shared variable store reference.
pub type ReadVariableConfig = Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>>;

/// A convenience node that reads variable values from a shared VariableNode store.
///
/// This node provides a simpler interface for reading variables compared to using
/// VariableNode directly. It requires the variable store to be configured via the
/// configuration port.
pub struct ReadVariableNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current configuration state.
  current_config: Arc<Mutex<Option<ReadVariableConfig>>>,
}

impl ReadVariableNode {
  /// Creates a new ReadVariableNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::ReadVariableNode;
  ///
  /// let node = ReadVariableNode::new("read_var".to_string());
  /// // Creates ports: configuration, name → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "name".to_string()],
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
impl Node for ReadVariableNode {
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
      let name_stream = inputs.remove("name").ok_or("Missing 'name' input")?;

      // Tag streams to distinguish config from data
      let config_stream = config_stream.map(|item| (MessageType::Config, item));
      let name_stream = name_stream.map(|item| (MessageType::Data, item));

      // Merge streams
      let merged_stream = stream::select(config_stream, name_stream);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration - expect ReadVariableConfig
              if let Ok(arc_store) = item.clone().downcast::<ReadVariableConfig>() {
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_store));
              } else {
                // Try to downcast Arc<Arc<...>> pattern
                if let Ok(arc_arc_store) = item.clone().downcast::<Arc<ReadVariableConfig>>() {
                  *config_state_clone.lock().await = Some(Arc::clone(&**arc_arc_store));
                } else {
                  // Invalid configuration type
                  let error_msg =
                    "Invalid configuration type - expected ReadVariableConfig".to_string();
                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            MessageType::Data => {
              // Read operation: expect variable name as String
              match &*config_state_clone.lock().await {
                Some(store) => {
                  if let Ok(arc_name) = item.downcast::<String>() {
                    let var_name = (*arc_name).clone();
                    let vars = store.lock().await;
                    if let Some(value) = vars.get(&var_name) {
                      // Variable exists, send value to output
                      let _ = out_tx_clone.send(value.clone()).await;
                    } else {
                      // Variable doesn't exist, send error
                      let error_msg = format!("Variable '{}' not found", var_name);
                      let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                      let _ = error_tx_clone.send(error_arc).await;
                    }
                  } else {
                    // Invalid type for name
                    let error_msg = "Name port expects String (variable name)".to_string();
                    let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
                None => {
                  // No configuration set
                  let error_msg =
                    "No variable store configured. Please send configuration before reading."
                      .to_string();
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
