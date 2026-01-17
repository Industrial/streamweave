//! # Break Node
//!
//! A transform node that breaks from processing when a signal is received.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to forward
//! - **Input**: `"signal"` - Receives break signal (any value triggers break)
//! - **Output**: `"out"` - Sends items until break signal is received
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node forwards items from the "in" port to the "out" port until a signal
//! is received on the "signal" port. Once a signal is received, it stops
//! forwarding items (but continues to consume the input stream).
//! It supports:
//! - Forwarding items until break signal
//! - Any value on the signal port triggers a break
//! - Items received after break signal are dropped

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  Config,
  In,
  Signal,
}

/// A node that breaks from processing when a signal is received.
///
/// The node forwards items from the "in" port to the "out" port until
/// a signal is received on the "signal" port. Once a signal is received,
/// it stops forwarding items.
pub struct BreakNode {
  pub(crate) base: BaseNode,
}

impl BreakNode {
  /// Creates a new BreakNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `BreakNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::advanced::BreakNode;
  ///
  /// let node = BreakNode::new("break".to_string());
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
impl Node for BreakNode {
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

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(signal_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone(); // Unused for now, but kept for consistency

      tokio::spawn(async move {
        let mut merged_stream = merged_stream;
        let mut broken = false;

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Signal => {
              // Break signal received - stop forwarding items
              broken = true;
            }
            InputPort::In => {
              // Forward item only if not broken
              if !broken {
                let _ = out_tx_clone.send(item).await;
              }
              // If broken, we continue consuming but don't forward
            }
            InputPort::Config => {
              // Configuration port is unused for now
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
