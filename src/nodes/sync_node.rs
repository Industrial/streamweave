//! # Sync Node
//!
//! A synchronization node that waits for values from all input ports before emitting a combined output.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that specify the number of input ports and timeout
//! - **Input**: `"in_0"`, `"in_1"`, ..., `"in_n"` - Receives values from each input port (number of ports is configurable)
//! - **Output**: `"out"` - Sends combined output when all inputs are ready
//! - **Output**: `"error"` - Sends errors that occur during synchronization (e.g., timeout)
//!
//! ## Behavior
//!
//! The node acts as a barrier synchronization point. It waits for one value from each configured
//! input port. Once all values are received, it combines them into a single output (as a Vec)
//! and emits it on the "out" port. If a timeout is configured and not all inputs arrive within
//! the timeout period, an error is sent to the "error" port.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Configuration for SyncNode.
pub struct SyncConfig {
  /// Number of input ports to wait for (in_0, in_1, ..., in_n-1)
  pub num_inputs: usize,
  /// Optional timeout duration. If None, waits indefinitely.
  pub timeout: Option<Duration>,
}

/// A node that waits for values from all input ports before emitting a combined output.
///
/// This node provides barrier synchronization, ensuring that processing only proceeds
/// when all required inputs are available.
pub struct SyncNode {
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<SyncConfig>>>>,
}

impl SyncNode {
  /// Creates a new SyncNode with the given name and number of input ports.
  ///
  /// # Arguments
  ///
  /// * `name` - Name of the node
  /// * `num_inputs` - Number of input ports (in_0, in_1, ..., in_n-1)
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::SyncNode;
  ///
  /// let node = SyncNode::new("sync".to_string(), 3);
  /// // Creates ports: configuration, in_0, in_1, in_2 → out, error
  /// ```
  pub fn new(name: String, num_inputs: usize) -> Self {
    let mut input_ports = vec!["configuration".to_string()];
    for i in 0..num_inputs {
      input_ports.push(format!("in_{}", i));
    }

    Self {
      base: BaseNode::new(
        name,
        input_ports,
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
impl Node for SyncNode {
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
      // Extract configuration stream
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;

      // Extract all input streams (in_0, in_1, ..., in_n)
      // Tag each input stream with its port name and merge them
      let mut tagged_streams = Vec::new();
      let mut input_port_names = Vec::new();

      for (port_name, stream) in inputs {
        if port_name.starts_with("in_") {
          input_port_names.push(port_name.clone());
          let port_name_clone = port_name.clone();
          let tagged = stream.map(move |item| (port_name_clone.clone(), item));
          tagged_streams.push(Box::pin(tagged)
            as Pin<
              Box<dyn tokio_stream::Stream<Item = (String, Arc<dyn Any + Send + Sync>)> + Send>,
            >);
        }
      }

      let merged_inputs = stream::select_all(tagged_streams);

      // Tag config stream
      let config_stream = config_stream.map(|item| (MessageType::Config, item));

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process configuration
      let config_state_clone = Arc::clone(&config_state);
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut config_stream = config_stream;

        // Process configuration updates
        while let Some((msg_type, item)) = config_stream.next().await {
          match msg_type {
            MessageType::Config => {
              // Try to downcast configuration
              if let Ok(config) = item.downcast::<SyncConfig>() {
                *config_state_clone.lock().await = Some(config);
              } else {
                let error_msg = "Invalid configuration type - expected SyncConfig".to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              // Config stream should only have Config messages
            }
          }
        }
      });

      // Process input streams - wait for all inputs
      let out_tx_final = out_tx.clone();
      let error_tx_final = error_tx.clone();
      let config_state_final = Arc::clone(&config_state);

      tokio::spawn(async move {
        let mut merged = merged_inputs;
        let mut buffers: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();

        loop {
          // Get current configuration
          let config_opt = {
            let config_guard = config_state_final.lock().await;
            config_guard.clone()
          };

          if let Some(config) = config_opt {
            let num_inputs = config.num_inputs;
            let timeout_duration = config.timeout;

            // Wait for values from all input ports (with optional timeout)
            let wait_result = if let Some(timeout_dur) = timeout_duration {
              tokio::time::timeout(timeout_dur, async {
                while buffers.len() < num_inputs {
                  if let Some((port_name, value)) = merged.next().await {
                    buffers.insert(port_name, value);
                  } else {
                    // Stream ended
                    break;
                  }
                }
                buffers.len() == num_inputs
              })
              .await
            } else {
              // No timeout - wait indefinitely
              while buffers.len() < num_inputs {
                if let Some((port_name, value)) = merged.next().await {
                  buffers.insert(port_name, value);
                } else {
                  // Stream ended
                  break;
                }
              }
              Ok(buffers.len() == num_inputs)
            };

            // Check if timeout occurred
            let all_received = match wait_result {
              Ok(received) => received,
              Err(_) => {
                // Timeout occurred
                let error_msg =
                  "Synchronization timeout: not all inputs received within timeout period"
                    .to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_final.send(error_arc).await;
                buffers.clear();
                continue;
              }
            };

            // Check if we have all required inputs
            if all_received && buffers.len() == num_inputs {
              // All inputs received - combine and emit in order
              let mut combined: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
              let mut all_present = true;
              for i in 0..num_inputs {
                let port_name = format!("in_{}", i);
                if let Some(value) = buffers.remove(&port_name) {
                  combined.push(value);
                } else {
                  // Missing input - this shouldn't happen if buffers.len() == num_inputs
                  all_present = false;
                  let error_msg =
                    format!("Synchronization error: missing input from {}", port_name);
                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                  let _ = error_tx_final.send(error_arc).await;
                  break;
                }
              }

              if all_present && combined.len() == num_inputs {
                let combined_arc: Arc<dyn Any + Send + Sync> = Arc::new(combined);
                let _ = out_tx_final.send(combined_arc).await;
              }

              // Clear buffers for next iteration
              buffers.clear();
            }
          } else {
            // No configuration set yet - wait a bit and retry
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            continue;
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
