//! # Debounce Node
//!
//! A transform node that debounces a stream, emitting items only after a specified
//! period of inactivity.
#![allow(clippy::doc_lazy_continuation)]
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to debounce
//! - **Input**: `"delay"` - Receives debounce delay in milliseconds (numeric)
//! - **Output**: `"out"` - Sends debounced items after the delay period
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node implements classic debouncing behavior:
//! - When an item is received, any pending emission is cancelled
//! - A new timer is started for the specified delay period
//! - If no new items arrive before the timer expires, the item is emitted
//! - If a new item arrives, the process restarts with the new item
//! - Useful for handling rapid-fire events and reacting only to stable final values

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract u64 from various numeric types for delay in milliseconds.
///
/// Accepts:
/// - u64 directly
/// - Numeric types (i32, i64, u32) converted to u64
fn get_u64(value: &Arc<dyn Any + Send + Sync>) -> Result<u64, String> {
  if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    Ok(*arc_u64)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Delay cannot be negative".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Delay too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Delay cannot be negative".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Delay too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    Ok((*arc_u32).into())
  } else {
    Err(format!(
      "Unsupported type for delay: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  In,
  Delay,
}

/// A node that debounces a stream, emitting items only after a specified
/// period of inactivity.
///
/// The debounce delay is received on the "delay" port and can be:
/// - A numeric value (u64, i32, i64, u32) representing milliseconds
/// The node debounces items from the "in" port, emitting each item only after
/// the specified delay period has passed without receiving a new item.
pub struct DebounceNode {
  pub(crate) base: BaseNode,
}

impl DebounceNode {
  /// Creates a new DebounceNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `DebounceNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::DebounceNode;
  ///
  /// let node = DebounceNode::new("debounce".to_string());
  /// // Creates ports: configuration, in, delay → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "delay".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for DebounceNode {
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
      let delay_stream = inputs.remove("delay").ok_or("Missing 'delay' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let delay_stream = delay_stream.map(|item| (InputPort::Delay, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(delay_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      // Create output channels
      let (out_tx, out_rx) = mpsc::channel(10);
      let (error_tx, error_rx) = mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged_stream = merged_stream;
        let mut debounce_delay: Option<u64> = None;
        let mut pending_item: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut current_timer: Option<tokio::task::JoinHandle<()>> = None;

        // Function to emit an item after delay
        let emit_item =
          |item: Arc<dyn Any + Send + Sync>, out_tx: &mpsc::Sender<Arc<dyn Any + Send + Sync>>| {
            let out_tx = out_tx.clone();
            tokio::spawn(async move {
              let _ = out_tx.send(item).await;
            })
          };

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Delay => {
              // Update debounce delay
              match get_u64(&item) {
                Ok(delay) => {
                  debounce_delay = Some(delay);
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Cancel any existing timer
              if let Some(timer) = current_timer.take() {
                timer.abort();
              }

              // Store the new item
              pending_item = Some(item);

              // Start a new timer if we have a delay configured
              if let Some(delay_ms) = debounce_delay {
                let item_to_emit = pending_item.clone().unwrap();
                let out_tx_for_timer = out_tx_clone.clone();

                current_timer = Some(tokio::spawn(async move {
                  tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                  emit_item(item_to_emit, &out_tx_for_timer);
                }));
              }
            }
          }
        }

        // If there's still a pending item when the stream ends, emit it immediately
        // (since no more items will cancel the debounce)
        if let Some(item) = pending_item {
          if let Some(timer) = current_timer.take() {
            timer.abort();
          }
          emit_item(item, &out_tx_clone);
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
