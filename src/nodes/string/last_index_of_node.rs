//! # String Last Index Of Node
//!
//! A transform node that finds the last index of a substring within a string.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives string value to search in
//! - **Input**: `"substring"` - Receives substring to find
//! - **Output**: `"out"` - Sends the last index as i64 (-1 if not found)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node finds the last occurrence of a substring within a string and returns its index.
//! Returns -1 if the substring is not found. It supports:
//! - Case-sensitive substring search
//! - Returning -1 for not found substrings
//! - Finding the rightmost occurrence of the substring

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::string_last_index_of;
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
  Substring,
}

/// A node that finds the last index of a substring within a string.
pub struct StringLastIndexOfNode {
  pub(crate) base: BaseNode,
}

impl StringLastIndexOfNode {
  /// Creates a new StringLastIndexOfNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringLastIndexOfNode;
  ///
  /// let node = StringLastIndexOfNode::new("my_last_index_of".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "substring".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringLastIndexOfNode {
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
      let substring_stream = inputs
        .remove("substring")
        .ok_or("Missing 'substring' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let substring_stream = substring_stream.map(|item| (InputPort::Substring, item));

      // Merge streams
      let merged_stream = stream::select(in_stream, substring_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut substring_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Substring => {
              substring_buffer = Some(item);
            }
          }

          // Process when both inputs are available
          if let (Some(v), Some(substring)) = (in_buffer.as_ref(), substring_buffer.as_ref()) {
            match string_last_index_of(v, substring) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                substring_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                substring_buffer = None;
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
