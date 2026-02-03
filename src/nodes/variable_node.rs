//! # Variable Node
//!
//! A node that provides thread-safe graph-level variable storage for sharing state between nodes.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates (optional, for future extensibility)
//! - **Input**: `"read"` - Receives read requests (variable name as String)
//! - **Input**: `"write"` - Receives write requests (pair of variable name and value)
//! - **Input**: `"value"` - Receives values to write (used with write port)
//! - **Output**: `"out"` - Sends variable values in response to read requests
//! - **Output**: `"error"` - Sends errors that occur during read/write operations
//!
//! ## Behavior
//!
//! The node maintains a thread-safe HashMap of variable names to values. It supports:
//! - **Read operations**: Send a variable name (String) on the "read" port, receive the value on "out"
//! - **Write operations**: Send a variable name (String) on the "write" port and the value on "value" port
//!
//! Variables are stored as type-erased `Arc<dyn Any + Send + Sync>` for flexibility.

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

/// Configuration for VariableNode (currently unused, reserved for future extensibility).
pub struct VariableConfig {
  // Reserved for future configuration options
}

/// A node that provides thread-safe graph-level variable storage.
///
/// The node maintains a shared variable store that can be accessed by multiple nodes
/// in the graph. Variables are stored as type-erased values and must be downcast when retrieved.
pub struct VariableNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current configuration state.
  current_config: Arc<Mutex<Option<Arc<VariableConfig>>>>,
  /// Thread-safe variable storage
  variables: Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>>,
}

impl VariableNode {
  /// Creates a new VariableNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::VariableNode;
  ///
  /// let node = VariableNode::new("variables".to_string());
  /// // Creates ports: configuration, read, write, value → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "read".to_string(),
          "write".to_string(),
          "value".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
      variables: Arc::new(Mutex::new(HashMap::new())),
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

  /// Returns a clone of the variables store for sharing with other nodes.
  ///
  /// This allows other nodes to access the same variable store.
  pub fn variables(&self) -> Arc<Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>> {
    Arc::clone(&self.variables)
  }
}

#[async_trait]
impl Node for VariableNode {
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
    let variables = Arc::clone(&self.variables);

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;
      let read_stream = inputs.remove("read").ok_or("Missing 'read' input")?;
      let write_stream = inputs.remove("write").ok_or("Missing 'write' input")?;
      let value_stream = inputs.remove("value").ok_or("Missing 'value' input")?;

      // Tag streams to distinguish message types
      let config_stream =
        config_stream.map(|item| (MessageType::Config, "config".to_string(), item));
      let read_stream = read_stream.map(|item| (MessageType::Data, "read".to_string(), item));
      let write_stream = write_stream.map(|item| (MessageType::Data, "write".to_string(), item));
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
        Box::pin(read_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(write_stream)
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
      let variables_clone = Arc::clone(&variables);
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      // Track pending write operations using a queue (FIFO)
      // This allows multiple concurrent writes to be processed in order
      let pending_writes: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let pending_writes = pending_writes;

        while let Some((msg_type, port_name, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration (currently unused, but store for future use)
              if let Ok(arc_config) = item.clone().downcast::<Arc<VariableConfig>>() {
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_config));
              } else {
                // Configuration is optional, ignore invalid types
              }
            }
            MessageType::Data => {
              match port_name.as_str() {
                "read" => {
                  // Read operation: expect variable name as String
                  if let Ok(arc_name) = item.downcast::<String>() {
                    let var_name = (*arc_name).clone();
                    let vars = variables_clone.lock().await;
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
                    // Invalid type for read request
                    let error_msg = "Read port expects String (variable name)".to_string();
                    let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
                "write" => {
                  // Write operation: expect variable name as String
                  if let Ok(arc_name) = item.downcast::<String>() {
                    let var_name = (*arc_name).clone();
                    // Add to queue, wait for value on "value" port
                    pending_writes.lock().await.push_back(var_name);
                  } else {
                    // Invalid type for write request
                    let error_msg = "Write port expects String (variable name)".to_string();
                    let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
                "value" => {
                  // Value for write operation
                  // Check if there's a pending write (FIFO order)
                  let var_name_opt = {
                    let mut pending = pending_writes.lock().await;
                    pending.pop_front()
                  };
                  if let Some(var_name) = var_name_opt {
                    // Store the variable
                    let mut vars = variables_clone.lock().await;
                    vars.insert(var_name.clone(), item.clone());
                    drop(vars);

                    // Write successful, no output needed (or could send confirmation)
                  } else {
                    // No pending write, this is an error
                    let error_msg = "Value received but no pending write operation".to_string();
                    let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                    let _ = error_tx_clone.send(error_arc).await;
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
