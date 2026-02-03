//! # Array Sort Node
//!
//! A transform node that sorts the elements in an array.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives array value (Vec<Arc<dyn Any + Send + Sync>>)
//! - **Input**: `"order"` - Receives sort order ("ascending" or "descending", default: "ascending")
//! - **Output**: `"out"` - Sends the sorted array
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node sorts the elements in an array and returns the result. It supports:
//! - Ascending or descending order
//! - Type-aware comparison (uses compare_less_than for type promotion)
//! - Incompatible types are placed at the end
//! - Error handling: Non-array inputs result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::array::common::array_sort;
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
  In,
  Order,
}

/// A node that sorts the elements in an array.
///
/// The node receives an array value on the "in" port and a sort order on the "order" port,
/// then outputs the sorted array to the "out" port.
pub struct ArraySortNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl ArraySortNode {
  /// Creates a new ArraySortNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::array::ArraySortNode;
  ///
  /// let node = ArraySortNode::new("sort".to_string());
  /// // Creates ports: configuration, in, order → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "order".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for ArraySortNode {
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
      let order_stream = inputs.remove("order").ok_or("Missing 'order' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let order_stream = order_stream.map(|item| (InputPort::Order, item));

      // Merge streams
      let merged_stream = stream::select(in_stream, order_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut order_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Order => {
              order_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(v), Some(order)) = (in_buffer.as_ref(), order_buffer.as_ref()) {
            // Parse order (default to ascending)
            let ascending = if let Ok(arc_str) = order.clone().downcast::<String>() {
              arc_str.to_lowercase() != "descending"
            } else if let Ok(arc_bool) = order.clone().downcast::<bool>() {
              *arc_bool // true = ascending, false = descending
            } else {
              true // Default to ascending
            };

            match array_sort(v, ascending) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                order_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                order_buffer = None;
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
