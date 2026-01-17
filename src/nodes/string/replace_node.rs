//! # String Replace Node
//!
//! A transform node that replaces all occurrences of a pattern in a string with a replacement string.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives string value
//! - **Input**: `"pattern"` - Receives pattern string to search for
//! - **Input**: `"replacement"` - Receives replacement string
//! - **Output**: `"out"` - Sends the string with all occurrences replaced
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node replaces all occurrences of a pattern in a string with a replacement string. It supports:
//! - Literal string replacement (all occurrences)
//! - Pattern and replacement must be strings
//! - Error handling: Non-string inputs result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::string_replace;
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
  Pattern,
  Replacement,
}

/// A node that replaces all occurrences of a pattern in a string.
///
/// The node receives a string value on the "in" port, a pattern on the "pattern" port,
/// and a replacement on the "replacement" port, then outputs the string with all
/// occurrences of the pattern replaced to the "out" port.
pub struct StringReplaceNode {
  pub(crate) base: BaseNode,
}

impl StringReplaceNode {
  /// Creates a new StringReplaceNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringReplaceNode;
  ///
  /// let node = StringReplaceNode::new("replace".to_string());
  /// // Creates ports: configuration, in, pattern, replacement → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "pattern".to_string(),
          "replacement".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringReplaceNode {
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
      let pattern_stream = inputs.remove("pattern").ok_or("Missing 'pattern' input")?;
      let replacement_stream = inputs
        .remove("replacement")
        .ok_or("Missing 'replacement' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let pattern_stream = pattern_stream.map(|item| (InputPort::Pattern, item));
      let replacement_stream = replacement_stream.map(|item| (InputPort::Replacement, item));

      // Merge streams
      let merged_stream = stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(pattern_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(replacement_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
      ]);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut pattern_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut replacement_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Pattern => {
              pattern_buffer = Some(item);
            }
            InputPort::Replacement => {
              replacement_buffer = Some(item);
            }
          }

          // Process when all inputs are available
          if let (Some(v), Some(pattern), Some(replacement)) = (
            in_buffer.as_ref(),
            pattern_buffer.as_ref(),
            replacement_buffer.as_ref(),
          ) {
            match string_replace(v, pattern, replacement) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                pattern_buffer = None;
                replacement_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                pattern_buffer = None;
                replacement_buffer = None;
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
