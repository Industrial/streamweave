//! # Buffer Node
//!
//! A transform node that buffers items into batches of a specified size.
#![allow(clippy::doc_lazy_continuation)]
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to buffer
//! - **Input**: `"size"` - Receives buffer size (numeric: how many items per batch)
//! - **Output**: `"out"` - Sends batches of buffered items (Vec<Arc<dyn Any + Send + Sync>>)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node collects items from the input stream into batches of the specified size.
//! When the buffer reaches the specified size, it emits the complete batch and starts a new buffer.
//! If the stream ends with a partial batch, the remaining items are discarded.
//!
//! It supports:
//!
//! - Buffering items into fixed-size batches
//! - Configurable batch size via the "size" port
//! - Error handling: Invalid size values result in errors

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract usize from various numeric types for buffer size.
///
/// Accepts:
/// - usize directly
/// - Numeric types (i32, i64, u32, u64) converted to usize
fn get_usize(value: &Arc<dyn Any + Send + Sync>) -> Result<usize, String> {
  if let Ok(arc_usize) = value.clone().downcast::<usize>() {
    Ok(*arc_usize)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Buffer size cannot be negative".to_string());
    }
    if *arc_i32 == 0 {
      return Err("Buffer size cannot be zero".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Buffer size too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Buffer size cannot be negative".to_string());
    }
    if *arc_i64 == 0 {
      return Err("Buffer size cannot be zero".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Buffer size too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    if *arc_u32 == 0 {
      return Err("Buffer size cannot be zero".to_string());
    }
    (*arc_u32)
      .try_into()
      .map_err(|_| "Buffer size too large".to_string())
  } else if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    if *arc_u64 == 0 {
      return Err("Buffer size cannot be zero".to_string());
    }
    (*arc_u64)
      .try_into()
      .map_err(|_| "Buffer size too large".to_string())
  } else {
    Err(format!(
      "Unsupported type for buffer size: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  In,
  Size,
}

/// A node that buffers items into batches of a specified size.
///
/// The buffer size is received on the "size" port and can be:
/// - A numeric value (usize, i32, i64, u32, u64)
/// The node collects items from the "in" port into buffers of the specified size.
/// When a buffer is full, it emits the batch to the "out" port.
pub struct BufferNode {
  pub(crate) base: BaseNode,
}

impl BufferNode {
  /// Creates a new BufferNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `BufferNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::BufferNode;
  ///
  /// let node = BufferNode::new("buffer".to_string());
  /// // Creates ports: configuration, in, size → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "size".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for BufferNode {
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
      let size_stream = inputs.remove("size").ok_or("Missing 'size' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let size_stream = size_stream.map(|item| (InputPort::Size, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(size_stream)
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
        let mut buffer_size: Option<usize> = None;
        let mut data_items: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        // First, collect all inputs
        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Size => {
              // Set buffer size (use first valid value)
              if buffer_size.is_none() {
                match get_usize(&item) {
                  Ok(size) => {
                    buffer_size = Some(size);
                  }
                  Err(e) => {
                    let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                    let _ = error_tx_clone.send(error_arc).await;
                    return;
                  }
                }
              }
            }
            InputPort::In => {
              // Collect all data items
              data_items.push(item);
            }
          }
        }

        // Now process all collected data if we have a buffer size
        if let Some(size) = buffer_size {
          // Process items in batches
          for chunk in data_items.chunks(size) {
            if chunk.len() == size {
              // Only emit full batches
              let batch: Vec<Arc<dyn Any + Send + Sync>> = chunk.to_vec();
              let _ = out_tx_clone
                .send(Arc::new(batch) as Arc<dyn Any + Send + Sync>)
                .await;
            }
            // Partial chunks at the end are discarded
          }
        } else {
          // No buffer size configured - this is an error
          let error_msg = "No buffer size configured".to_string();
          let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
          let _ = error_tx_clone.send(error_arc).await;
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
