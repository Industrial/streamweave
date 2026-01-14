//! # Timeout Node
//!
//! A node that applies a timeout to each item from the input stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to process with timeout
//! - **Input**: `"timeout"` - Receives the timeout duration (in milliseconds as numeric value or Duration)
//! - **Output**: `"out"` - Sends items that arrived within the timeout
//! - **Output**: `"error"` - Sends timeout errors when items don't arrive in time
//!
//! ## Behavior
//!
//! The node waits for each item from the input stream with a timeout. If an item arrives
//! within the timeout duration, it is forwarded to the "out" port. If the timeout expires
//! before an item arrives, an error is sent to the "error" port.
//! The timeout can be provided as:
//! - Numeric values (i32, i64, u32, u64, f64) interpreted as milliseconds
//! - Duration type directly

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
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

/// A node that applies a timeout to each item from the input stream.
///
/// The timeout is received on the "timeout" port and can be:
/// - A Duration type directly
/// - A numeric value (i32, i64, u32, u64, f64) interpreted as milliseconds
/// For each item from the "in" port, the node waits with the specified timeout.
/// If the item arrives within the timeout, it is forwarded to "out".
/// If the timeout expires, an error is sent to "error".
pub struct TimeoutNode {
  pub(crate) base: BaseNode,
}

impl TimeoutNode {
  /// Creates a new TimeoutNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::time::TimeoutNode;
  ///
  /// let node = TimeoutNode::new("timeout".to_string());
  /// // Creates ports: configuration, in, timeout → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "timeout".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for TimeoutNode {
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
      let timeout_stream = inputs.remove("timeout").ok_or("Missing 'timeout' input")?;

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process streams: first get timeout, then process in_stream with timeout
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut timeout_stream = timeout_stream;
        let mut in_stream = in_stream;
        let mut timeout_duration: Option<Duration> = None;

        // First, get the timeout duration
        while timeout_duration.is_none() {
          if let Some(item) = timeout_stream.next().await {
            match extract_duration(&item) {
              Ok(dur) => {
                timeout_duration = Some(dur);
                break;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                return;
              }
            }
          } else {
            // Timeout stream ended without providing timeout
            let error_arc =
              Arc::new("Timeout duration not provided".to_string()) as Arc<dyn Any + Send + Sync>;
            let _ = error_tx_clone.send(error_arc).await;
            return;
          }
        }

        // Now process in_stream with timeout for each item
        if let Some(timeout_dur) = timeout_duration {
          loop {
            match tokio::time::timeout(timeout_dur, in_stream.next()).await {
              Ok(Some(item)) => {
                // Item arrived within timeout
                let _ = out_tx_clone.send(item).await;
              }
              Ok(None) => {
                // Stream ended normally
                break;
              }
              Err(_) => {
                // Timeout occurred waiting for next item
                let error_arc = Arc::new(format!("Timeout after {:?}", timeout_dur))
                  as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                break;
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
