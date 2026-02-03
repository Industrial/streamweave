//! # String Split Node
//!
//! A transform node that splits a string by a delimiter and returns an array of strings.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives string value
//! - **Input**: `"delimiter"` - Receives delimiter string
//! - **Output**: `"out"` - Sends the array of split strings (Vec<Arc<dyn Any + Send + Sync>>)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node splits a string by a delimiter and returns an array of strings. It supports:
//! - Literal string delimiter splitting
//! - Returns Vec<Arc<dyn Any + Send + Sync>> where each element is a String
//! - Error handling: Non-string inputs result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::string_split;
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
  Delimiter,
}

/// A node that splits a string by a delimiter.
///
/// The node receives a string value on the "in" port and a delimiter on the "delimiter" port,
/// then outputs an array of strings (Vec<Arc<dyn Any + Send + Sync>>) to the "out" port.
pub struct StringSplitNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl StringSplitNode {
  /// Creates a new StringSplitNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringSplitNode;
  ///
  /// let node = StringSplitNode::new("split".to_string());
  /// // Creates ports: configuration, in, delimiter → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "delimiter".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringSplitNode {
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
      let delimiter_stream = inputs
        .remove("delimiter")
        .ok_or("Missing 'delimiter' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let delimiter_stream = delimiter_stream.map(|item| (InputPort::Delimiter, item));

      // Merge streams
      let merged_stream = stream::select(in_stream, delimiter_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut delimiter_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Delimiter => {
              delimiter_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(v), Some(delimiter)) = (in_buffer.as_ref(), delimiter_buffer.as_ref()) {
            match string_split(v, delimiter) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                delimiter_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                delimiter_buffer = None;
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
