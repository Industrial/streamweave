//! # Min Aggregate Node
//!
//! An aggregation node that finds the minimum value in a stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives numeric values to find minimum
//! - **Output**: `"out"` - Sends the minimum value
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., empty stream)
//!
//! ## Behavior
//!
//! The node finds the minimum value from the input stream and outputs a single result
//! when the stream ends. It supports:
//! - Integer types: i32, i64, u32, u64
//! - Floating point types: f32, f64
//! - Type promotion: smaller types are promoted to larger types when needed
//! - Empty streams: returns an error (minimum of empty stream is undefined)
//! - Error handling: Empty streams result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::comparison::common::compare_less_than;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that finds the minimum value in a stream.
///
/// The node receives numeric values on the "in" port and outputs
/// the minimum value to the "out" port when the stream ends.
pub struct MinAggregateNode {
  pub(crate) base: BaseNode,
}

impl MinAggregateNode {
  /// Creates a new MinAggregateNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::aggregation::MinAggregateNode;
  ///
  /// let node = MinAggregateNode::new("min".to_string());
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
impl Node for MinAggregateNode {
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

      // Process the input stream and find the minimum
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut minimum: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some(item) = in_stream.next().await {
          match &minimum {
            None => {
              // First item becomes the minimum
              minimum = Some(item);
            }
            Some(current_min) => {
              // Compare current item with current minimum
              match compare_less_than(&item, current_min) {
                Ok(less_than_arc) => {
                  if let Ok(less_than) = less_than_arc.downcast::<bool>()
                    && *less_than
                  {
                    // Current item is less than current minimum, update minimum
                    minimum = Some(item);
                  }
                  // Otherwise keep current minimum
                  // If downcast fails, ignore the comparison (compare_less_than handles incompatible types)
                }
                Err(_) => {
                  // Comparison error - ignore this item or could send error
                  // For now, we'll just ignore incompatible types
                }
              }
            }
          }
        }

        // Emit the minimum (or error if stream was empty)
        match minimum {
          Some(min_value) => {
            let _ = out_tx_clone.send(min_value).await;
          }
          None => {
            // Empty stream: minimum is undefined
            let error_msg = "Cannot find minimum of empty stream".to_string();
            let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
            let _ = error_tx_clone.send(error_arc).await;
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
