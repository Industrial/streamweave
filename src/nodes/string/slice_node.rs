//! # String Slice Node
//!
//! A transform node that extracts a substring from a string using start and end indices.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives string value
//! - **Input**: `"start"` - Receives start index (usize, i32, i64, u32, or u64)
//! - **Input**: `"end"` - Receives end index (usize, i32, i64, u32, or u64)
//! - **Output**: `"out"` - Sends the extracted substring
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., invalid indices, type mismatch)
//!
//! ## Behavior
//!
//! The node extracts a substring from a string using start and end indices. It supports:
//! - String slicing with various numeric index types
//! - Bounds checking
//! - Invalid index range detection (start > end, out of bounds)
//! - Error handling: Non-string inputs or invalid indices result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::string::common::string_slice;
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
  Start,
  End,
}

/// A node that extracts a substring from a string input.
///
/// The node receives a string value on the "in" port and start/end indices
/// on the "start" and "end" ports, then outputs the extracted substring
/// to the "out" port.
pub struct StringSliceNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl StringSliceNode {
  /// Creates a new StringSliceNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::string::StringSliceNode;
  ///
  /// let node = StringSliceNode::new("slice".to_string());
  /// // Creates ports: configuration, in, start, end → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "start".to_string(),
          "end".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for StringSliceNode {
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
      let start_stream = inputs.remove("start").ok_or("Missing 'start' input")?;
      let end_stream = inputs.remove("end").ok_or("Missing 'end' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let start_stream = start_stream.map(|item| (InputPort::Start, item));
      let end_stream = end_stream.map(|item| (InputPort::End, item));

      // Merge streams
      let merged_stream = stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(start_stream)
          as Pin<
            Box<dyn tokio_stream::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >,
        Box::pin(end_stream)
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
        let mut start_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut end_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Start => {
              start_buffer = Some(item);
            }
            InputPort::End => {
              end_buffer = Some(item);
            }
          }

          // Process when all inputs are available
          if let (Some(v), Some(start), Some(end)) = (
            in_buffer.as_ref(),
            start_buffer.as_ref(),
            end_buffer.as_ref(),
          ) {
            match string_slice(v, start, end) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                start_buffer = None;
                end_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                start_buffer = None;
                end_buffer = None;
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
