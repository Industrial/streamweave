//! # Average Node
//!
//! An aggregation node that calculates the average (mean) of numeric values in a stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives numeric values to average
//! - **Output**: `"out"` - Sends the average of all values
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch, empty stream)
//!
//! ## Behavior
//!
//! The node calculates the average of all numeric values from the input stream and outputs
//! a single result when the stream ends. It supports:
//! - Integer types: i32, i64, u32, u64 (with overflow checking)
//! - Floating point types: f32, f64
//! - Type promotion: smaller types are promoted to larger types when needed
//! - Empty streams: returns an error (average of empty stream is undefined)
//! - Error handling: Non-numeric inputs and empty streams result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::arithmetic::common::{add_values, divide_values};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that calculates the average of numeric values from a stream.
///
/// The node receives numeric values on the "in" port and outputs
/// the average of all values to the "out" port when the stream ends.
pub struct AverageNode {
  pub(crate) base: BaseNode,
}

impl AverageNode {
  /// Creates a new AverageNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::aggregation::AverageNode;
  ///
  /// let node = AverageNode::new("average".to_string());
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
impl Node for AverageNode {
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

      // Process the input stream and calculate the average
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut sum: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut count: i32 = 0;

        while let Some(item) = in_stream.next().await {
          match &sum {
            None => {
              // First item becomes the sum
              sum = Some(item);
              count = 1;
            }
            Some(acc_sum) => {
              // Add current item to sum
              match add_values(acc_sum, &item) {
                Ok(new_sum) => {
                  sum = Some(new_sum);
                  count += 1;
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

        // Calculate average: sum / count
        if count == 0 {
          // Empty stream: average is undefined
          let error_msg = "Cannot calculate average of empty stream".to_string();
          let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
          let _ = error_tx_clone.send(error_arc).await;
          return;
        }

        if let Some(sum_value) = sum {
          // Convert count to f64 for division
          let count_f64 = count as f64;
          let count_arc = Arc::new(count_f64) as Arc<dyn Any + Send + Sync>;

          // Convert integer sums to f64 for accurate average calculation
          let sum_f64 = if let Ok(arc_i32) = sum_value.clone().downcast::<i32>() {
            Arc::new(*arc_i32 as f64) as Arc<dyn Any + Send + Sync>
          } else if let Ok(arc_i64) = sum_value.clone().downcast::<i64>() {
            Arc::new(*arc_i64 as f64) as Arc<dyn Any + Send + Sync>
          } else if let Ok(arc_u32) = sum_value.clone().downcast::<u32>() {
            Arc::new(*arc_u32 as f64) as Arc<dyn Any + Send + Sync>
          } else if let Ok(arc_u64) = sum_value.clone().downcast::<u64>() {
            Arc::new(*arc_u64 as f64) as Arc<dyn Any + Send + Sync>
          } else if let Ok(arc_f32) = sum_value.clone().downcast::<f32>() {
            Arc::new(*arc_f32 as f64) as Arc<dyn Any + Send + Sync>
          } else {
            // Already f64 or other type - use as-is
            sum_value
          };

          match divide_values(&sum_f64, &count_arc) {
            Ok(average) => {
              let _ = out_tx_clone.send(average).await;
            }
            Err(e) => {
              let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
              let _ = error_tx_clone.send(error_arc).await;
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
