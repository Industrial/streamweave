//! # Throttle Node
//!
//! A transform node that throttles a stream to limit emission rate.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to throttle
//! - **Input**: `"period"` - Receives throttle period in milliseconds (numeric)
//! - **Output**: `"out"` - Sends throttled items (at most one per period)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node enforces a minimum time interval between emitted items.
//! When an item is received:
//! - If the throttle period has elapsed since the last emission, emit immediately
//! - If the throttle period has not elapsed, the item is dropped (not buffered)
//!
//! This ensures that items are not emitted more frequently than the specified rate,
//! protecting downstream consumers from being overwhelmed by rapid-fire events.
#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract u64 from various numeric types for period in milliseconds.
///
/// Accepts:
/// - u64 directly
/// - Numeric types (i32, i64, u32) converted to u64
fn get_u64(value: &Arc<dyn Any + Send + Sync>) -> Result<u64, String> {
  if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    Ok(*arc_u64)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Period cannot be negative".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Period too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Period cannot be negative".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Period too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    Ok((*arc_u32).into())
  } else {
    Err(format!(
      "Unsupported type for period: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  In,
  Period,
}

/// A node that throttles a stream to limit the emission rate.
///
/// The node ensures that items are not emitted more frequently than
/// the specified throttle period, dropping items that arrive too quickly.
pub struct ThrottleNode {
  pub(crate) base: BaseNode,
}

impl ThrottleNode {
  /// Creates a new ThrottleNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `ThrottleNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::ThrottleNode;
  ///
  /// let node = ThrottleNode::new("throttle".to_string());
  /// // Creates ports: configuration, in, period → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "period".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for ThrottleNode {
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
      let period_stream = inputs.remove("period").ok_or("Missing 'period' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let period_stream = period_stream.map(|item| (InputPort::Period, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(period_stream)
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
        let mut throttle_period: Option<u64> = None;
        let mut last_emission: Option<Instant> = None;
        let mut buffered_items: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Period => {
              // Update throttle period
              match get_u64(&item) {
                Ok(period) => {
                  throttle_period = Some(period);
                  // Process any buffered items now that we have a period
                  for buffered_item in buffered_items.drain(..) {
                    let now = Instant::now();
                    let can_emit = if let Some(last) = last_emission {
                      now.duration_since(last) >= Duration::from_millis(period)
                    } else {
                      // First item can always be emitted
                      true
                    };

                    if can_emit {
                      // Emit the item and update last emission time
                      let _ = out_tx_clone.send(buffered_item).await;
                      last_emission = Some(now);
                    }
                    // Items that can't be emitted are silently dropped
                  }
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Check if we can emit this item
              if let Some(period_ms) = throttle_period {
                let now = Instant::now();
                let can_emit = if let Some(last) = last_emission {
                  now.duration_since(last) >= Duration::from_millis(period_ms)
                } else {
                  // First item can always be emitted
                  true
                };

                if can_emit {
                  // Emit the item and update last emission time
                  let _ = out_tx_clone.send(item).await;
                  last_emission = Some(now);
                }
                // Items that can't be emitted are silently dropped
              } else {
                // Buffer items until period is configured
                buffered_items.push(item);
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
