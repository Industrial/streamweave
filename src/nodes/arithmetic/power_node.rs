//! # Power Node
//!
//! A transform node that performs exponentiation (power) operation on two numeric inputs.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"base"` - Receives the base value
//! - **Input**: `"exponent"` - Receives the exponent value
//! - **Output**: `"out"` - Sends the result of (base ^ exponent)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., overflow, type mismatch)
//!
//! ## Behavior
//!
//! The node performs exponentiation on two numeric values. It supports:
//! - Integer types: i32, i64, u32, u64 (exponent must be non-negative for integer operations)
//! - Floating point types: f32, f64 (any exponent allowed)
//! - Type promotion: smaller types are promoted to larger types when needed
//! - Overflow detection: integer operations check for overflow and emit errors

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::arithmetic::common::power_values;
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  /// Base value.
  Base,
  /// Exponent value.
  Exponent,
}

/// A node that performs exponentiation (power) operation on two numeric inputs.
///
/// The node receives two numeric values on "base" and "exponent" ports and outputs
/// the result of the power operation to the "out" port.
pub struct PowerNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl PowerNode {
  /// Creates a new PowerNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::arithmetic::PowerNode;
  ///
  /// let node = PowerNode::new("power".to_string());
  /// // Creates ports: configuration, base, exponent → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "base".to_string(),
          "exponent".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for PowerNode {
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
      let base_stream = inputs.remove("base").ok_or("Missing 'base' input")?;
      let exponent_stream = inputs
        .remove("exponent")
        .ok_or("Missing 'exponent' input")?;

      // Tag streams to distinguish inputs
      let base_stream = base_stream.map(|item| (InputPort::Base, item));
      let exponent_stream = exponent_stream.map(|item| (InputPort::Exponent, item));

      // Merge streams
      let merged_stream = stream::select(base_stream, exponent_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut base_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut exponent_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::Base => {
              base_buffer = Some(item);
            }
            InputPort::Exponent => {
              exponent_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(base), Some(exponent)) = (base_buffer.as_ref(), exponent_buffer.as_ref()) {
            match power_values(base, exponent) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                base_buffer = None;
                exponent_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                base_buffer = None;
                exponent_buffer = None;
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
