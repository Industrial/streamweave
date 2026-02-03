//! # String Pad Node
//!
//! A transform node that pads a string to a specified length with a padding character.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives string value
//! - **Input**: `"length"` - Receives target length (numeric)
//! - **Input**: `"padding"` - Receives padding character (String or char, defaults to space)
//! - **Input**: `"side"` - Receives padding side ("left", "right", or "center", defaults to "right")
//! - **Output**: `"out"` - Sends the padded string
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., invalid side, type mismatch)
//!
//! ## Behavior
//!
//! The node pads a string to a specified length with a padding character. It supports:
//! - Left padding: Pads on the left side
//! - Right padding: Pads on the right side (default)
//! - Center padding: Pads on both sides (preferring left if odd)
//! - Default padding character: space if not provided
//! - If string is already longer than target length, returns as-is
//! - Error handling: Invalid side or non-string inputs result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::string_pad;
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
  Length,
  Padding,
  Side,
}

/// A node that pads a string to a specified length with a padding character.
///
/// The node receives a string value, target length, padding character, and side
/// on their respective ports, then outputs the padded string to the "out" port.
pub struct StringPadNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl StringPadNode {
  /// Creates a new StringPadNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringPadNode;
  ///
  /// let node = StringPadNode::new("pad".to_string());
  /// // Creates ports: configuration, in, length, padding, side → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "length".to_string(),
          "padding".to_string(),
          "side".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringPadNode {
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
      let length_stream = inputs.remove("length").ok_or("Missing 'length' input")?;
      let padding_stream = inputs.remove("padding").ok_or("Missing 'padding' input")?;
      let side_stream = inputs.remove("side").ok_or("Missing 'side' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let length_stream = length_stream.map(|item| (InputPort::Length, item));
      let padding_stream = padding_stream.map(|item| (InputPort::Padding, item));
      let side_stream = side_stream.map(|item| (InputPort::Side, item));

      // Merge streams
      let merged_stream = stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(length_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(padding_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(side_stream)
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
        let mut length_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut padding_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut side_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Length => {
              length_buffer = Some(item);
            }
            InputPort::Padding => {
              padding_buffer = Some(item);
            }
            InputPort::Side => {
              side_buffer = Some(item);
            }
          }

          // Process when all inputs are available
          if let (Some(v), Some(length), Some(padding), Some(side)) = (
            in_buffer.as_ref(),
            length_buffer.as_ref(),
            padding_buffer.as_ref(),
            side_buffer.as_ref(),
          ) {
            match string_pad(v, length, padding, side) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                length_buffer = None;
                padding_buffer = None;
                side_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                length_buffer = None;
                padding_buffer = None;
                side_buffer = None;
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
