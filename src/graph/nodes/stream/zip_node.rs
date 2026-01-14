//! # Zip Node
//!
//! A transform node that zips multiple streams together.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in_0"`, `"in_1"`, ..., `"in_n"` - Receives data items from multiple streams
//! - **Output**: `"out"` - Sends zipped items (arrays of items from each stream)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node zips multiple streams together by taking one item from each stream
//! and combining them into an array. The zipping stops when any stream ends
//! (shortest stream determines the length). Each output is an array containing
//! one item from each input stream in order.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that zips multiple streams together.
///
/// The node receives items from multiple input streams (in_0, in_1, ..., in_n)
/// and zips them together by taking one item from each stream and combining
/// them into an array. The zipping stops when any stream ends.
pub struct ZipNode {
  pub(crate) base: BaseNode,
}

impl ZipNode {
  /// Creates a new ZipNode with the given name and number of input streams.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  /// * `num_inputs` - The number of input streams to zip (creates in_0, in_1, ..., in_n-1)
  ///
  /// # Returns
  ///
  /// A new `ZipNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::stream::ZipNode;
  ///
  /// let node = ZipNode::new("zip".to_string(), 2);
  /// // Creates ports: configuration, in_0, in_1 → out, error
  /// ```
  pub fn new(name: String, num_inputs: usize) -> Self {
    let mut input_ports = vec!["configuration".to_string()];
    for i in 0..num_inputs {
      input_ports.push(format!("in_{}", i));
    }

    Self {
      base: BaseNode::new(
        name,
        input_ports,
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for ZipNode {
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
      // Extract configuration stream
      let _config_stream = inputs.remove("configuration");

      // Extract all input streams (in_0, in_1, ..., in_n)
      let mut input_streams: Vec<(usize, crate::graph::node::InputStream)> = Vec::new();
      let mut input_indices = Vec::new();

      // Collect all input streams and their indices
      for (port_name, stream) in inputs {
        if port_name.starts_with("in_") {
          if let Ok(index) = port_name[3..].parse::<usize>() {
            input_indices.push(index);
            input_streams.push((index, stream));
          }
        }
      }

      // Sort by index to ensure correct order
      input_streams.sort_by_key(|(idx, _)| *idx);

      if input_streams.is_empty() {
        return Err("No input streams found (expected in_0, in_1, ...)".into());
      }

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process streams by zipping them
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        // Convert streams to a vector of pinned streams
        let mut streams: Vec<Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>> =
          input_streams
            .into_iter()
            .map(|(_, stream)| Box::pin(stream) as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>)
            .collect();

        // Zip streams: take one item from each stream until any stream ends
        loop {
          let mut zipped_items = Vec::new();
          let mut all_have_items = true;

          // Try to get one item from each stream
          for stream in &mut streams {
            match stream.next().await {
              Some(item) => {
                zipped_items.push(item);
              }
              None => {
                // This stream has ended, so zipping stops
                all_have_items = false;
                break;
              }
            }
          }

          if !all_have_items {
            // At least one stream ended, stop zipping
            break;
          }

          // All streams had items, create zipped array
          if zipped_items.len() == streams.len() {
            let zipped_array = Arc::new(zipped_items) as Arc<dyn Any + Send + Sync>;
            let _ = out_tx_clone.send(zipped_array).await;
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

