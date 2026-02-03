//! # String Prepend Node
//!
//! A transform node that prepends a prefix to a string.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives base string value
//! - **Input**: `"prefix"` - Receives prefix to prepend
//! - **Output**: `"out"` - Sends the result (prefix + base)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node prepends a prefix to a base string. It supports:
//! - String + String: Direct concatenation (prefix + base)
//! - Error handling: Non-string inputs result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::prepend_prefix;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  In,
  Prefix,
}

/// A node that prepends a prefix to a string.
pub struct StringPrependNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl StringPrependNode {
  /// Creates a new StringPrependNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringPrependNode;
  ///
  /// let node = StringPrependNode::new("my_prepend".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "prefix".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringPrependNode {
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
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let prefix_stream = inputs.remove("prefix").ok_or("Missing 'prefix' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let prefix_stream = prefix_stream.map(|item| (InputPort::Prefix, item));

      // Merge streams
      let merged_stream = stream::select(in_stream, prefix_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut prefix_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Prefix => {
              prefix_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(v), Some(prefix)) = (in_buffer.as_ref(), prefix_buffer.as_ref()) {
            match prepend_prefix(prefix, v) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                prefix_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                prefix_buffer = None;
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
