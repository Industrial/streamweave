//! # Not Node
//!
//! A transform node that performs logical NOT operation on a boolean input.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives boolean value
//! - **Output**: `"out"` - Sends the result of (NOT in)
//! - **Output**: `"error"` - Sends errors that occur during processing

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::boolean_logic::common::to_bool;
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::StreamExt;

/// A node that performs logical NOT operation on a boolean input.
///
/// The node receives a boolean value on the "in" port and outputs
/// the negated result to the "out" port.
pub struct NotNode {
  pub(crate) base: BaseNode,
}

impl NotNode {
  /// Creates a new NotNode with the given name.
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
impl Node for NotNode {
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
      // Extract input stream (configuration port is present but unused for now)
      let _config_stream = inputs.remove("configuration");
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the input stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut input = input_stream;
        while let Some(item) = input.next().await {
          match to_bool(&item) {
            Ok(b) => {
              let result = !b;
              let result_arc = Arc::new(result) as Arc<dyn Any + Send + Sync>;
              let _ = out_tx_clone.send(result_arc).await;
            }
            Err(e) => {
              let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
              let _ = error_tx_clone.send(error_arc).await;
            }
          }
        }
      });

      // Convert channels to streams
      use tokio_stream::wrappers::ReceiverStream;
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
