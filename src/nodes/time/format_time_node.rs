//! # Format Time Node
//!
//! A transform node that formats timestamps into human-readable strings.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives timestamp as i64 (milliseconds since Unix epoch)
//! - **Input**: `"format"` - Receives optional format string (defaults to RFC3339 if not provided)
//! - **Output**: `"out"` - Sends formatted time string
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node formats timestamps into human-readable strings using chrono formatting.
//! If no format string is provided, it defaults to RFC3339 format.
//! Supports standard chrono format specifiers.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use futures::FutureExt;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Formats a timestamp into a human-readable string.
///
/// This function attempts to convert an i64 timestamp (milliseconds since Unix epoch)
/// into a formatted string using the provided format string, or RFC3339 if none provided.
///
/// Returns the formatted string as `Arc<dyn Any + Send + Sync>` or an error string.
fn format_timestamp(
  timestamp: &Arc<dyn Any + Send + Sync>,
  format: Option<&Arc<dyn Any + Send + Sync>>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast timestamp to i64
  let arc_timestamp = timestamp.clone().downcast::<i64>().map_err(|_| {
    format!(
      "Unsupported type for timestamp input: {} (input must be i64)",
      std::any::type_name_of_val(&**timestamp)
    )
  })?;

  // Convert milliseconds to seconds and nanoseconds
  let timestamp_ms = *arc_timestamp;
  let secs = timestamp_ms / 1000;
  let nsecs = ((timestamp_ms % 1000) * 1_000_000) as u32;

  // Create UTC DateTime from timestamp
  let datetime = Utc
    .timestamp_opt(secs, nsecs)
    .single()
    .ok_or_else(|| format!("Invalid timestamp: {}", timestamp_ms))?;

  // Get format string or use default
  let format_str = if let Some(format_arc) = format {
    // Try to downcast format to String
    let arc_format = format_arc.clone().downcast::<String>().map_err(|_| {
      format!(
        "Unsupported type for format input: {} (format must be String)",
        std::any::type_name_of_val(&**format_arc)
      )
    })?;
    (*arc_format).clone()
  } else {
    // Default to RFC3339 format
    "%Y-%m-%dT%H:%M:%S%.3fZ".to_string()
  };

  // Format the datetime
  let formatted = datetime.format(&format_str).to_string();
  Ok(Arc::new(formatted) as Arc<dyn Any + Send + Sync>)
}

/// A node that formats timestamps into human-readable strings.
///
/// The node receives timestamps on the "in" port and optional format strings on the "format" port,
/// then outputs formatted time strings to the "out" port.
pub struct FormatTimeNode {
  pub(crate) base: BaseNode,
}

impl FormatTimeNode {
  /// Creates a new FormatTimeNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::time::FormatTimeNode;
  ///
  /// let node = FormatTimeNode::new("format_time".to_string());
  /// // Creates ports: configuration, in, format → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "format".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for FormatTimeNode {
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
      let mut in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let mut format_stream = inputs.remove("format");

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the streams
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut current_format: Option<Arc<dyn Any + Send + Sync>> = None;

        // First, try to get an initial format if available
        if let Some(ref mut stream) = format_stream.as_mut()
          && let Some(Some(item)) = stream.next().now_or_never()
        {
          current_format = Some(item);
        }

        // Process timestamps
        while let Some(item) = in_stream.next().await {
          // Check for format updates before processing each timestamp
          if let Some(ref mut stream) = format_stream.as_mut()
            && let Some(Some(format_item)) = stream.next().now_or_never()
          {
            current_format = Some(format_item);
          }

          // Format the timestamp using current format (or default)
          match format_timestamp(&item, current_format.as_ref()) {
            Ok(result) => {
              let _ = out_tx_clone.send(result).await;
            }
            Err(e) => {
              let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
              let _ = error_tx_clone.send(error_arc).await;
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
