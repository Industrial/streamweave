//! # WriteVariable Node
//!
//! A convenience node that writes variable values to a shared VariableNode store.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives the variable store reference (Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>>)
//! - **Input**: `"name"` - Receives variable names (String) to write
//! - **Input**: `"value"` - Receives variable values to write
//! - **Output**: `"out"` - Sends confirmation or the written value (optional)
//! - **Output**: `"error"` - Sends errors that occur during write operations
//!
//! ## Behavior
//!
//! The node writes variable values to a shared variable store. The store reference must be
//! provided via the configuration port. For each variable name received on the "name" port,
//! the node waits for the corresponding value on the "value" port and stores it in the
//! variable store. The write operation uses a FIFO queue to match names with values.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Configuration for WriteVariableNode - contains the shared variable store reference.
pub type WriteVariableConfig = Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>>;

/// A convenience node that writes variable values to a shared VariableNode store.
///
/// This node provides a simpler interface for writing variables compared to using
/// VariableNode directly. It requires the variable store to be configured via the
/// configuration port.
pub struct WriteVariableNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current configuration state.
  current_config: Arc<Mutex<Option<WriteVariableConfig>>>,
}

impl WriteVariableNode {
  /// Creates a new WriteVariableNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::WriteVariableNode;
  ///
  /// let node = WriteVariableNode::new("write_var".to_string());
  /// // Creates ports: configuration, name, value → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "name".to_string(),
          "value".to_string(),
        ],
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
impl Node for WriteVariableNode {
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
      let value_stream = inputs.remove("value").ok_or("Missing 'value' input")?;

      // Tag streams to distinguish message types
      let config_stream =
        config_stream.map(|item| (MessageType::Config, "config".to_string(), item));
      let name_stream = name_stream.map(|item| (MessageType::Data, "name".to_string(), item));
      let value_stream = value_stream.map(|item| (MessageType::Data, "value".to_string(), item));

      // Merge all streams
      let merged_stream = stream::select_all(vec![
        Box::pin(config_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(name_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(value_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
      ]);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      // Track pending write operations using a queue (FIFO)
      let pending_writes: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let pending_writes = pending_writes;

        while let Some((msg_type, port_name, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration - expect WriteVariableConfig
              if let Ok(arc_store) = item.clone().downcast::<WriteVariableConfig>() {
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_store));
              } else {
                // Try to downcast Arc<Arc<...>> pattern
                if let Ok(arc_arc_store) = item.clone().downcast::<Arc<WriteVariableConfig>>() {
                  *config_state_clone.lock().await = Some(Arc::clone(&**arc_arc_store));
                } else {
                  // Invalid configuration type
                  let error_msg =
                    "Invalid configuration type - expected WriteVariableConfig".to_string();
                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            MessageType::Data => {
              match port_name.as_str() {
                "name" => {
                  // Write operation: expect variable name as String
                  match &*config_state_clone.lock().await {
                    Some(_store) => {
                      if let Ok(arc_name) = item.downcast::<String>() {
                        let var_name = (*arc_name).clone();
                        // Add to queue, wait for value on "value" port
                        pending_writes.lock().await.push_back(var_name);
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
                        "No variable store configured. Please send configuration before writing."
                          .to_string();
                      let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                      let _ = error_tx_clone.send(error_arc).await;
                    }
                  }
                }
                "value" => {
                  // Value for write operation
                  match &*config_state_clone.lock().await {
                    Some(store) => {
                      // Check if there's a pending write (FIFO order)
                      let var_name_opt = {
                        let mut pending = pending_writes.lock().await;
                        pending.pop_front()
                      };
                      if let Some(var_name) = var_name_opt {
                        // Store the variable
                        let mut vars = store.lock().await;
                        vars.insert(var_name.clone(), item.clone());
                        drop(vars);

                        // Write successful, send confirmation to output
                        let _ = out_tx_clone.send(item).await;
                      } else {
                        // No pending write, this is an error
                        let error_msg = "Value received but no pending write operation".to_string();
                        let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                        let _ = error_tx_clone.send(error_arc).await;
                      }
                    }
                    None => {
                      // No configuration set
                      let error_msg =
                        "No variable store configured. Please send configuration before writing."
                          .to_string();
                      let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                      let _ = error_tx_clone.send(error_arc).await;
                    }
                  }
                }
                _ => {}
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
