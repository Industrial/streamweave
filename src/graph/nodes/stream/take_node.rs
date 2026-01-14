//! # Take Node
//!
//! A transform node that takes the first N items from the input stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items
//! - **Input**: `"count"` - Receives count value (numeric: how many items to take)
//! - **Output**: `"out"` - Sends the first N items
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node takes the first N items from the input stream and forwards them to the output.
//! After taking N items, it stops forwarding items (but continues to consume the stream).
//! It supports:
//! - Taking items based on a numeric count (usize, i32, i64, u32, u64)
//! - Error handling: Invalid count values (negative, wrong type) result in errors sent to the error port

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
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
      return Err("Count cannot be negative".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Count too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Count cannot be negative".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Count too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    (*arc_u32)
      .try_into()
      .map_err(|_| "Count too large".to_string())
  } else if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    (*arc_u64)
      .try_into()
      .map_err(|_| "Count too large".to_string())
  } else {
    Err(format!(
      "Unsupported type for count: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  Config,
  In,
  Count,
}

/// A node that takes the first N items from the input stream.
///
/// The count is received on the "count" port and can be:
/// - A numeric value (usize, i32, i64, u32, u64)
/// The node forwards the first N items from the "in" port to the "out" port,
/// then stops forwarding (but continues consuming the stream).
pub struct TakeNode {
  pub(crate) base: BaseNode,
}

impl TakeNode {
  /// Creates a new TakeNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `TakeNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::stream::TakeNode;
  ///
  /// let node = TakeNode::new("take".to_string());
  /// // Creates ports: configuration, in, count → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "count".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for TakeNode {
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
      let count_stream = inputs.remove("count").ok_or("Missing 'count' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let count_stream = count_stream.map(|item| (InputPort::Count, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(count_stream)
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
        let mut count: Option<usize> = None;
        let mut items_taken: usize = 0;

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Configuration port is unused for now
            }
            InputPort::Count => {
              // Update count
              match get_usize(&item) {
                Ok(c) => {
                  count = Some(c);
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Forward item if we haven't reached the count limit
              if let Some(c) = count {
                if items_taken < c {
                  let _ = out_tx_clone.send(item).await;
                  items_taken += 1;
                }
                // If we've reached the limit, we continue consuming but don't forward
              } else {
                // No count set yet - buffer or error?
                // For now, we'll wait for count to be set before processing items
                // This means items arriving before count will be dropped
                // Alternative: buffer items until count is received
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
