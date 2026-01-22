//! # Parse Time Node
//!
//! A transform node that parses time strings into timestamps.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives time string to parse
//! - **Input**: `"format"` - Receives optional format string (defaults to RFC3339 if not provided)
//! - **Output**: `"out"` - Sends parsed timestamp as i64 (milliseconds since Unix epoch)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node parses time strings using chrono format specifiers and returns timestamps.
//! If no format string is provided, it attempts to parse common formats automatically.
//! Returns the timestamp as i64 milliseconds since Unix epoch.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Parses a time string into a timestamp.
///
/// This function attempts to parse a time string using the provided format string,
/// or tries common formats if none provided.
///
/// Returns the timestamp as `Arc<dyn Any + Send + Sync>` (i64) or an error string.
fn parse_time_string(
  time_str: &Arc<dyn Any + Send + Sync>,
  format: Option<&Arc<dyn Any + Send + Sync>>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast time string to String
  let arc_time_str = time_str.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for time string input: {} (input must be String)",
      std::any::type_name_of_val(&**time_str)
    )
  })?;

  let time_string = (*arc_time_str).clone();

  // Try to parse with provided format first
  if let Some(format_arc) = format {
    // Try to downcast format to String
    let arc_format = format_arc.clone().downcast::<String>().map_err(|_| {
      format!(
        "Unsupported type for format input: {} (format must be String)",
        std::any::type_name_of_val(&**format_arc)
      )
    })?;

    let format_str = (*arc_format).clone();

    // Try to parse with the specified format
    if let Ok(datetime) = DateTime::parse_from_str(&time_string, &format_str) {
      let timestamp_ms = datetime.timestamp_millis();
      return Ok(Arc::new(timestamp_ms) as Arc<dyn Any + Send + Sync>);
    } else if let Ok(naive) = NaiveDateTime::parse_from_str(&time_string, &format_str) {
      let datetime = Utc.from_utc_datetime(&naive);
      let timestamp_ms = datetime.timestamp_millis();
      return Ok(Arc::new(timestamp_ms) as Arc<dyn Any + Send + Sync>);
    } else {
      return Err(format!(
        "Failed to parse '{}' with format '{}'",
        time_string, format_str
      ));
    }
  }

  // Try common formats if no format provided
  let common_formats = vec![
    "%Y-%m-%dT%H:%M:%S%.3fZ", // RFC3339 with milliseconds
    "%Y-%m-%dT%H:%M:%SZ",     // RFC3339
    "%Y-%m-%d %H:%M:%S",      // ISO-like
    "%Y/%m/%d %H:%M:%S",      // Alternative ISO
    "%Y-%m-%d",               // Date only
    "%H:%M:%S",               // Time only
  ];

  for format_str in common_formats {
    if let Ok(datetime) = DateTime::parse_from_str(&time_string, format_str) {
      let timestamp_ms = datetime.timestamp_millis();
      return Ok(Arc::new(timestamp_ms) as Arc<dyn Any + Send + Sync>);
    } else if let Ok(naive) = NaiveDateTime::parse_from_str(&time_string, format_str) {
      let datetime = Utc.from_utc_datetime(&naive);
      let timestamp_ms = datetime.timestamp_millis();
      return Ok(Arc::new(timestamp_ms) as Arc<dyn Any + Send + Sync>);
    }
  }

  Err(format!(
    "Failed to parse '{}' with any common format",
    time_string
  ))
}

/// A node that parses time strings into timestamps.
///
/// The node receives time strings on the "in" port and optional format strings on the "format" port,
/// then outputs timestamps to the "out" port.
pub struct ParseTimeNode {
  pub(crate) base: BaseNode,
}

impl ParseTimeNode {
  /// Creates a new ParseTimeNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::time::ParseTimeNode;
  ///
  /// let node = ParseTimeNode::new("parse_time".to_string());
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
impl Node for ParseTimeNode {
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
        // For each timestamp, wait for its corresponding format (if any)
        while let Some(timestamp_item) = in_stream.next().await {
          // Try to get a format for this timestamp
          let format_item = if let Some(ref mut stream) = format_stream.as_mut() {
            // Use tokio::time::timeout to avoid blocking indefinitely
            match tokio::time::timeout(tokio::time::Duration::from_millis(10), stream.next()).await
            {
              Ok(Some(format)) => Some(format),
              _ => None, // No format available or timeout
            }
          } else {
            None
          };

          // Parse the time string using the format (or auto-detect if None)
          match parse_time_string(&timestamp_item, format_item.as_ref()) {
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
