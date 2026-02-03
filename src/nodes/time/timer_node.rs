//! # Timer Node
//!
//! A source node that generates periodic events at a specified interval.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"interval"` - Receives the interval duration (in milliseconds as numeric value or Duration)
//! - **Output**: `"out"` - Sends periodic timer events
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node generates periodic events at the specified interval. Each event is a timestamp
//! (as i64 milliseconds since epoch) or a simple counter. The interval can be provided as:
//! - Numeric values (i32, i64, u32, u64, f64) interpreted as milliseconds
//! - Duration type directly
//!   The node continues generating events until the interval stream ends or an error occurs.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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

/// A node that generates periodic events at a specified interval.
///
/// The interval is received on the "interval" port and can be:
/// - A Duration type directly
/// - A numeric value (i32, i64, u32, u64, f64) interpreted as milliseconds
///   The node generates events (timestamps as i64 milliseconds since epoch)
///   at the specified interval.
pub struct TimerNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl TimerNode {
  /// Creates a new TimerNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::time::TimerNode;
  ///
  /// let node = TimerNode::new("timer".to_string());
  /// // Creates ports: configuration, interval → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "interval".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for TimerNode {
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
      let interval_stream = inputs
        .remove("interval")
        .ok_or("Missing 'interval' input")?;

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process interval stream and generate periodic events
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut interval_stream = interval_stream;
        #[allow(unused_assignments)]
        let mut interval_duration: Option<Duration> = None;

        // First, get the interval duration
        if let Some(item) = interval_stream.next().await {
          match extract_duration(&item) {
            Ok(dur) => {
              interval_duration = Some(dur);
            }
            Err(e) => {
              let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
              let _ = error_tx_clone.send(error_arc).await;
              return;
            }
          }
        } else {
          // Interval stream ended without providing interval
          let error_arc =
            Arc::new("Interval duration not provided".to_string()) as Arc<dyn Any + Send + Sync>;
          let _ = error_tx_clone.send(error_arc).await;
          return;
        }

        // Generate periodic events
        if let Some(interval_dur) = interval_duration {
          let mut interval = tokio::time::interval(interval_dur);
          interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

          #[allow(clippy::never_loop)]
          loop {
            interval.tick().await;

            // Generate timestamp (milliseconds since epoch)
            let timestamp = SystemTime::now()
              .duration_since(UNIX_EPOCH)
              .unwrap()
              .as_millis() as i64;

            let event_arc = Arc::new(timestamp) as Arc<dyn Any + Send + Sync>;
            if out_tx_clone.send(event_arc).await.is_err() {
              // Receiver dropped, stop generating
              break;
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
