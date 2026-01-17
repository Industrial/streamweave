//! # Equal Node
//!
//! A transform node that performs equality comparison on two inputs.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in1"` - Receives first value
//! - **Input**: `"in2"` - Receives second value
//! - **Output**: `"out"` - Sends the result of (in1 == in2) as a boolean
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node performs equality comparison on two values. It supports:
//! - Integer types: i32, i64, u32, u64
//! - Floating point types: f32, f64 (uses epsilon comparison for floating point equality)
//! - String types
//! - Boolean types
//! - Type promotion: smaller types are promoted to larger types when needed
//! - Incompatible types: returns false (no error) for incompatible type combinations

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::comparison::common::compare_equal;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  In1,
  In2,
}

/// A node that performs equality comparison on two inputs.
///
/// The node receives two values on "in1" and "in2" ports and outputs
/// the result of the equality comparison (as a boolean) to the "out" port.
pub struct EqualNode {
  pub(crate) base: BaseNode,
}

impl EqualNode {
  /// Creates a new EqualNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::comparison::EqualNode;
  ///
  /// let node = EqualNode::new("equal".to_string());
  /// // Creates ports: configuration, in1, in2 → out, error
  /// ```
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
impl Node for EqualNode {
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
            match compare_equal(v1, v2) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in1_buffer = None;
                in2_buffer = None;
              }
              Err(e) => {
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
