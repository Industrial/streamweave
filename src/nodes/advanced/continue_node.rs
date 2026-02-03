//! # Continue Node
#![allow(clippy::type_complexity)]
//!
//! A transform node that skips the current item when a continue signal is received.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to forward
//! - **Input**: `"signal"` - Receives continue signal (any value triggers skip of current item)
//! - **Output**: `"out"` - Sends items, skipping those when continue signal is received
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node forwards items from the "in" port to the "out" port. When a signal
//! is received on the "signal" port, it skips forwarding the next item from the
//! "in" port (if any). The signal applies to the next item received after the signal.
//! It supports:
//! - Forwarding items normally
//! - Skipping items when continue signal is received
//! - Any value on the signal port triggers a skip of the next item

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;

use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  /// Input data port.
  In,
  /// Signal port for continue command.
  Signal,
}

/// A node that skips the current item when a continue signal is received.
///
/// The node forwards items from the "in" port to the "out" port. When a signal
/// is received on the "signal" port, it skips forwarding the next item from the
/// "in" port (if any).
pub struct ContinueNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl ContinueNode {
  /// Creates a new ContinueNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `ContinueNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::advanced::ContinueNode;
  ///
  /// let node = ContinueNode::new("continue".to_string());
  /// // Creates ports: configuration, in, signal → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "signal".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for ContinueNode {
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
      // Extract input streams
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let signal_stream = inputs.remove("signal").ok_or("Missing 'signal' input")?;

      // Create output channels
      let (out_tx, out_rx) = mpsc::channel(10);
      let (error_tx, error_rx) = mpsc::channel(10);

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let signal_stream = signal_stream.map(|item| (InputPort::Signal, item));

      // Process streams with fair scheduling
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone(); // Unused for now, but kept for consistency

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut signal_stream = signal_stream;
        let mut skip_next = false;

        loop {
          tokio::select! {
            // Check for signals
            signal_result = signal_stream.next() => {
              if let Some(_signal_item) = signal_result {
                // Continue signal received - skip the next item
                skip_next = true;
              }
            }
            // Check for input items
            in_result = in_stream.next() => {
              match in_result {
                Some((_, item)) => {
                  // Forward item only if not skipping
                  if skip_next {
                    skip_next = false; // Reset skip flag
                    // Skip this item - don't forward it
                  } else {
                    let _ = out_tx_clone.send(item).await;
                  }
                }
                None => {
                  // Input stream ended, check if signal stream is also ended
                  if signal_stream.next().await.is_none() {
                    break; // Both streams ended
                  }
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
