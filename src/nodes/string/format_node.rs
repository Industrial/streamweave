//! # String Format Node
//!
//! A transform node that formats strings with placeholders using values.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"template"` - Receives format template string (e.g., "Hello {}", "User: {}")
//! - **Input**: `"value"` - Receives value to substitute in template
//! - **Output**: `"out"` - Sends formatted result string
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch, invalid format)
//!
//! ## Behavior
//!
//! The node formats strings using Rust's format! macro. It supports:
//! - Single placeholder substitution: "{}" in template replaced with value
//! - Type conversion: Values are converted to strings for formatting
//! - Error handling: Invalid templates or non-convertible values result in errors

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::format_string;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  Template,
  Value,
}

/// A node that formats strings with placeholders.
pub struct StringFormatNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl StringFormatNode {
  /// Creates a new StringFormatNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringFormatNode;
  ///
  /// let node = StringFormatNode::new("my_format".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "template".to_string(),
          "value".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringFormatNode {
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
      // Extract input streams (configuration port is present but unused for now)
      let _config_stream = inputs.remove("configuration");
      let template_stream = inputs
        .remove("template")
        .ok_or("Missing 'template' input")?;
      let value_stream = inputs.remove("value").ok_or("Missing 'value' input")?;

      // Tag streams to distinguish inputs
      let template_stream = template_stream.map(|item| (InputPort::Template, item));
      let value_stream = value_stream.map(|item| (InputPort::Value, item));

      // Merge streams
      let merged_stream = stream::select(template_stream, value_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut template_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut value_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::Template => {
              template_buffer = Some(item);
            }
            InputPort::Value => {
              value_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(template), Some(value)) = (template_buffer.as_ref(), value_buffer.as_ref()) {
            match format_string(template, value) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                template_buffer = None;
                value_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                template_buffer = None;
                value_buffer = None;
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
