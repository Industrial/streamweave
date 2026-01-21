//! # First Node
//!
//! A transform node that emits only the first item from a stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items from which to extract the first
//! - **Output**: `"out"` - Sends the first item received (only once)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node tracks whether it has already emitted an item. When the first item
//! is received on the "in" port, it is immediately emitted on the "out" port.
//! All subsequent items are ignored.
//!
//! This is useful for extracting the first occurrence of an event, getting
//! the initial value from a stream, or sampling the first item in a sequence.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that emits only the first item from a stream.
///
/// After emitting the first item, all subsequent items are silently ignored.
pub struct FirstNode {
  pub(crate) base: BaseNode,
}

impl FirstNode {
  /// Creates a new FirstNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `FirstNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::FirstNode;
  ///
  /// let node = FirstNode::new("first".to_string());
  /// // Creates ports: configuration, in → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for FirstNode {
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

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the input stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut has_emitted = false;

        while let Some(item) = in_stream.next().await {
          if !has_emitted {
            // This is the first item - emit it
            let _ = out_tx_clone.send(item).await;
            has_emitted = true;
            // Note: We continue processing to drain the stream,
            // but we don't emit any more items
          }
          // All subsequent items are ignored
        }

        // Drop transmitters to signal end of stream
        drop(out_tx_clone);
        drop(error_tx_clone);
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
