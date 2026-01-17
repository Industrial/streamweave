//! # Sum Node
//!
//! An aggregation node that sums numeric values from a stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives numeric values to sum
//! - **Output**: `"out"` - Sends the sum of all values
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch, overflow)
//!
//! ## Behavior
//!
//! The node sums all numeric values from the input stream and outputs a single result
//! when the stream ends. It supports:
//! - Integer types: i32, i64, u32, u64 (with overflow checking)
//! - Floating point types: f32, f64
//! - Type promotion: smaller types are promoted to larger types when needed
//! - Empty streams: returns 0 (i32)
//! - Error handling: Non-numeric inputs result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::arithmetic::common::add_values;
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that sums numeric values from a stream.
///
/// The node receives numeric values on the "in" port and outputs
/// the sum of all values to the "out" port when the stream ends.
pub struct SumNode {
  pub(crate) base: BaseNode,
}

impl SumNode {
  /// Creates a new SumNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::aggregation::SumNode;
  ///
  /// let node = SumNode::new("sum".to_string());
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
impl Node for SumNode {
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

      // Process the input stream and accumulate the sum
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut accumulator: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some(item) = in_stream.next().await {
          match &accumulator {
            None => {
              // First item becomes the accumulator
              accumulator = Some(item);
            }
            Some(acc) => {
              // Add current item to accumulator
              match add_values(acc, &item) {
                Ok(sum) => {
                  accumulator = Some(sum);
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                  return; // Stop processing on error
                }
              }
            }
          }
        }

        // Emit the final sum (or 0 if stream was empty)
        match accumulator {
          Some(sum) => {
            let _ = out_tx_clone.send(sum).await;
          }
          None => {
            // Empty stream: return 0
            let zero = Arc::new(0i32) as Arc<dyn Any + Send + Sync>;
            let _ = out_tx_clone.send(zero).await;
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
