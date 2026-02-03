//! # Current Time Node
//!
//! A source node that outputs the current timestamp.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"trigger"` - Receives trigger signals to generate timestamps
//! - **Output**: `"out"` - Sends the current timestamp as i64 milliseconds since Unix epoch
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node generates the current timestamp whenever it receives a trigger signal.
//! The timestamp is represented as i64 milliseconds since Unix epoch.
//! This is a source node that can be used to timestamp events or generate time-based data.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  /// Trigger signal port.
  Trigger,
}

/// A source node that outputs the current timestamp.
pub struct CurrentTimeNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl CurrentTimeNode {
  /// Creates a new CurrentTimeNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::time::CurrentTimeNode;
  ///
  /// let node = CurrentTimeNode::new("current_time".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "trigger".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for CurrentTimeNode {
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
    Box::pin(async move {
      // Extract input streams (configuration port is present but unused for now)
      let _config_stream = inputs.remove("configuration");
      let trigger_stream = inputs.remove("trigger").ok_or("Missing 'trigger' input")?;

      // Tag streams to distinguish inputs
      let trigger_stream = trigger_stream.map(|item| (InputPort::Trigger, item));

      // Merge streams
      let merged_stream = stream::select(trigger_stream, stream::empty());

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;

        while let Some((port, _item)) = merged.next().await {
          match port {
            InputPort::Trigger => {
              // Generate current timestamp
              match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => {
                  let timestamp = duration.as_millis() as i64;
                  let timestamp_arc = Arc::new(timestamp) as Arc<dyn Any + Send + Sync>;
                  let _ = out_tx_clone.send(timestamp_arc).await;
                }
                Err(e) => {
                  let error_arc = Arc::new(format!("Failed to get current time: {}", e))
                    as Arc<dyn Any + Send + Sync>;
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
