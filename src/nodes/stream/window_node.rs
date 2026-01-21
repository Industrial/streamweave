//! # Window Node
//!
//! A transform node that collects items into fixed-size windows and emits them as batches.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to window
//! - **Input**: `"size"` - Receives window size (number of items per window)
//! - **Output**: `"out"` - Sends windows as `Vec<Arc<dyn Any + Send + Sync>>`
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node collects incoming items into windows of the specified size.
//! When a window reaches the configured size, it is emitted as a vector.
//! The node continues collecting items for subsequent windows.
//!
//! If the stream ends with a partial window (fewer items than the window size),
//! the partial window is discarded and not emitted.
#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract usize from various numeric types for window size.
///
/// Accepts:
/// - usize directly
/// - Numeric types (i32, i64, u32, u64) converted to usize
fn get_usize(value: &Arc<dyn Any + Send + Sync>) -> Result<usize, String> {
  if let Ok(arc_usize) = value.clone().downcast::<usize>() {
    Ok(*arc_usize)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Window size cannot be negative".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Window size too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Window size cannot be negative".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Window size too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    Ok((*arc_u32) as usize)
  } else if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    (*arc_u64)
      .try_into()
      .map_err(|_| "Window size too large".to_string())
  } else {
    Err(format!(
      "Unsupported type for window size: {} (must be numeric)",
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

/// A node that collects items into fixed-size windows and emits them as batches.
///
/// The node buffers incoming items until the window size is reached,
/// then emits the complete window as a vector of items.
pub struct WindowNode {
  pub(crate) base: BaseNode,
}

impl WindowNode {
  /// Creates a new WindowNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `WindowNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::WindowNode;
  ///
  /// let node = WindowNode::new("window".to_string());
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
impl Node for WindowNode {
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
        let mut window_size: Option<usize> = None;
        let mut current_window: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Size => {
              // Update window size
              match get_usize(&item) {
                Ok(size) => {
                  window_size = Some(size);
                  // If we have a valid size and buffered items, try to emit windows
                  if size > 0 && !current_window.is_empty() {
                    while current_window.len() >= size {
                      let window: Vec<Arc<dyn Any + Send + Sync>> =
                        current_window.drain(..size).collect();
                      let _ = out_tx_clone
                        .send(Arc::new(window) as Arc<dyn Any + Send + Sync>)
                        .await;
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
              // Add item to current window
              current_window.push(item);

              // If we have a valid window size, check if window is complete
              if let Some(size) = window_size
                && size > 0
                && current_window.len() >= size
              {
                let window: Vec<Arc<dyn Any + Send + Sync>> =
                  current_window.drain(..size).collect();
                let _ = out_tx_clone
                  .send(Arc::new(window) as Arc<dyn Any + Send + Sync>)
                  .await;
              }
            }
          }
        }

        // Drop transmitters to signal end of stream
        drop(out_tx_clone);
        drop(error_tx_clone);
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
