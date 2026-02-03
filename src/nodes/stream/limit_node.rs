//! # Limit Node
//!
//! A transform node that limits the stream to a maximum size.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items
//! - **Input**: `"max_size"` - Receives max size value (numeric: maximum number of items to forward)
//! - **Output**: `"out"` - Sends items up to the max size limit
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node limits the stream to a maximum size. After forwarding max_size items,
//! it stops forwarding items (but continues to consume the stream).
//! This is similar to TakeNode but with a different port name for clarity.
//! It supports:
//! - Limiting items based on a numeric max_size (usize, i32, i64, u32, u64)
//! - Error handling: Invalid max_size values (negative, wrong type) result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract usize from various numeric types.
///
/// Accepts:
/// - usize directly
/// - Numeric types (i32, i64, u32, u64) converted to usize
fn get_usize(value: &Arc<dyn Any + Send + Sync>) -> Result<usize, String> {
  if let Ok(arc_usize) = value.clone().downcast::<usize>() {
    Ok(*arc_usize)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Max size cannot be negative".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Max size too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Max size cannot be negative".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Max size too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    (*arc_u32)
      .try_into()
      .map_err(|_| "Max size too large".to_string())
  } else if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    (*arc_u64)
      .try_into()
      .map_err(|_| "Max size too large".to_string())
  } else {
    Err(format!(
      "Unsupported type for max_size: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  /// Input stream.
  In,
  /// Maximum buffer size.
  MaxSize,
}

/// A node that limits the stream to a maximum size.
///
/// The max_size is received on the "max_size" port and can be:
/// - A numeric value (usize, i32, i64, u32, u64)
///   The node forwards items from the "in" port to the "out" port up to the max_size limit,
///   then stops forwarding (but continues consuming the stream).
pub struct LimitNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl LimitNode {
  /// Creates a new LimitNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `LimitNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::LimitNode;
  ///
  /// let node = LimitNode::new("limit".to_string());
  /// // Creates ports: configuration, in, max_size → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "max_size".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for LimitNode {
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
      let max_size_stream = inputs
        .remove("max_size")
        .ok_or("Missing 'max_size' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let max_size_stream = max_size_stream.map(|item| (InputPort::MaxSize, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(max_size_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged_stream = merged_stream;
        let mut max_size: Option<usize> = None;
        let mut items_forwarded: usize = 0;
        let mut buffered_items: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::MaxSize => {
              // Update max_size
              match get_usize(&item) {
                Ok(size) => {
                  max_size = Some(size);
                  // Process any buffered items now that we have configuration
                  while let Some(buffered_item) = buffered_items.pop() {
                    if items_forwarded < size {
                      let _ = out_tx_clone.send(buffered_item).await;
                      items_forwarded += 1;
                    } else {
                      break;
                    }
                  }
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Forward item if we haven't reached the max_size limit
              if let Some(max) = max_size {
                if items_forwarded < max {
                  let _ = out_tx_clone.send(item).await;
                  items_forwarded += 1;
                }
                // If we've reached the limit, we continue consuming but don't forward
              } else {
                // Buffer items until max_size is set
                buffered_items.push(item);
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
