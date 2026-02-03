//! # Delay Node
//!
//! A node that delays each item from the input stream by a specified duration before forwarding it.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to delay
//! - **Input**: `"duration"` - Receives the delay duration (in milliseconds as numeric value or Duration)
//! - **Output**: `"out"` - Sends delayed items
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node delays each item from the input stream by the specified duration before forwarding it.
//! The duration can be provided as:
//! - Numeric values (i32, i64, u32, u64, f64) interpreted as milliseconds
//! - Duration type directly
//!
//! Each item is delayed independently, preserving order.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract Duration from various numeric types.
///
/// Accepts:
/// - Duration directly
/// - Numeric types (i32, i64, u32, u64, f64) interpreted as milliseconds
fn extract_duration(value: &Arc<dyn Any + Send + Sync>) -> Result<Duration, String> {
  // Try Duration directly
  if let Ok(arc_duration) = value.clone().downcast::<Duration>() {
    return Ok(*arc_duration);
  }

  // Try numeric types (interpreted as milliseconds)
  if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Duration cannot be negative".to_string());
    }
    return Ok(Duration::from_millis(*arc_i32 as u64));
  }

  if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Duration cannot be negative".to_string());
    }
    if *arc_i64 > (u64::MAX as i64) {
      return Err("Duration too large".to_string());
    }
    return Ok(Duration::from_millis(*arc_i64 as u64));
  }

  if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    return Ok(Duration::from_millis(*arc_u32 as u64));
  }

  if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    return Ok(Duration::from_millis(*arc_u64));
  }

  if let Ok(arc_f64) = value.clone().downcast::<f64>() {
    if *arc_f64 < 0.0 {
      return Err("Duration cannot be negative".to_string());
    }
    // Convert to milliseconds (truncate to u64)
    let millis = *arc_f64 as u64;
    return Ok(Duration::from_millis(millis));
  }

  Err(format!(
    "Unsupported type for duration: {} (must be Duration or numeric)",
    std::any::type_name_of_val(&**value)
  ))
}

/// Enum to tag input ports
#[allow(dead_code)]
enum InputPort {
  Config,
  In,
  Duration,
}

/// A node that delays each item from the input stream by a specified duration.
///
/// The duration is received on the "duration" port and can be:
/// - A Duration type directly
/// - A numeric value (i32, i64, u32, u64, f64) interpreted as milliseconds
///
/// Each item from the "in" port is delayed by the duration before being forwarded to "out".
pub struct DelayNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl DelayNode {
  /// Creates a new DelayNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::time::DelayNode;
  ///
  /// let node = DelayNode::new("delay".to_string());
  /// // Creates ports: configuration, in, duration → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "duration".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for DelayNode {
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
      let duration_stream = inputs
        .remove("duration")
        .ok_or("Missing 'duration' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let duration_stream = duration_stream.map(|item| (InputPort::Duration, item));

      // Merge streams
      #[allow(clippy::type_complexity)]
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(duration_stream)
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
        let mut duration: Option<Duration> = None;

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Configuration port is unused for now
            }
            InputPort::Duration => {
              // Update duration
              match extract_duration(&item) {
                Ok(dur) => {
                  duration = Some(dur);
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Delay and forward item
              if let Some(dur) = duration {
                // Delay by the duration
                tokio::time::sleep(dur).await;
                let _ = out_tx_clone.send(item).await;
              } else {
                // No duration set yet - forward immediately (or could error)
                // For now, forward immediately to avoid blocking
                let _ = out_tx_clone.send(item).await;
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
