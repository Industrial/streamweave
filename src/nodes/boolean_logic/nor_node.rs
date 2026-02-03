//! # Nor Node
//!
//! A transform node that performs logical NOR (NOT OR) operation on two boolean inputs.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in1"` - Receives first boolean value
//! - **Input**: `"in2"` - Receives second boolean value
//! - **Output**: `"out"` - Sends the result of (NOT (in1 OR in2))
//! - **Output**: `"error"` - Sends errors that occur during processing

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::boolean_logic::common::to_bool;
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::StreamExt;

/// A node that performs logical NOR operation on two boolean inputs.
///
/// The node receives two boolean values on "in1" and "in2" ports and outputs
/// the result of the NOR operation to the "out" port.
pub struct NorNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl NorNode {
  /// Creates a new NorNode with the given name.
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in1".to_string(),
          "in2".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for NorNode {
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
      let in1_stream = inputs.remove("in1").ok_or("Missing 'in1' input")?;
      let in2_stream = inputs.remove("in2").ok_or("Missing 'in2' input")?;

      // Tag streams to distinguish inputs
      let in1_stream = in1_stream.map(|item| (InputPort::In1, item));
      let in2_stream = in2_stream.map(|item| (InputPort::In2, item));

      // Merge streams
      let merged_stream = stream::select(in1_stream, in2_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in1_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut in2_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In1 => {
              in1_buffer = Some(item);
            }
            InputPort::In2 => {
              in2_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(v1), Some(v2)) = (in1_buffer.as_ref(), in2_buffer.as_ref()) {
            match (to_bool(v1), to_bool(v2)) {
              (Ok(b1), Ok(b2)) => {
                let result = !(b1 || b2);
                let result_arc = Arc::new(result) as Arc<dyn Any + Send + Sync>;
                let _ = out_tx_clone.send(result_arc).await;
                // Clear buffers after processing
                in1_buffer = None;
                in2_buffer = None;
              }
              (Err(e), _) | (_, Err(e)) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in1_buffer = None;
                in2_buffer = None;
              }
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

/// Enum to tag input ports.
enum InputPort {
  /// First input operand.
  In1,
  /// Second input operand.
  In2,
}
