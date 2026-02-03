//! # Logarithm Node
//!
//! A transform node that computes the logarithm of a value with a given base.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives the value to compute the logarithm of
//! - **Input**: `"base"` - Receives the base of the logarithm
//! - **Output**: `"out"` - Sends the logarithm result (f64)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., invalid inputs, type mismatch)
//!
//! ## Behavior
//!
//! The node computes log_base(value). It supports:
//! - Integer types: i32, i64, u32, u64 (converted to f64)
//! - Floating point types: f32, f64
//! - Returns result as f64
//! - Emits error for non-positive values or base <= 0 or base == 1

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::math::common::log_values;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  In,
  Base,
}

/// A node that computes the logarithm of a value with a given base.
///
/// The node receives a value on "in" and a base on "base" ports and outputs
/// the logarithm result to the "out" port as f64.
pub struct LogNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl LogNode {
  /// Creates a new LogNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::math::LogNode;
  ///
  /// let node = LogNode::new("log".to_string());
  /// // Creates ports: configuration, in, base → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "base".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for LogNode {
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
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let base_stream = inputs.remove("base").ok_or("Missing 'base' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let base_stream = base_stream.map(|item| (InputPort::Base, item));

      // Merge streams
      let merged_stream = stream::select(in_stream, base_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut base_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Base => {
              base_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(v), Some(b)) = (in_buffer.as_ref(), base_buffer.as_ref()) {
            match log_values(v, b) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                base_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                base_buffer = None;
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
