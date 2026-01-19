//! # Interleave Node
//!
//! A transform node that interleaves items from multiple streams.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in_0"`, `"in_1"`, ..., `"in_n"` - Receives data items from multiple streams
//! - **Output**: `"out"` - Sends interleaved items (one from each stream in round-robin order)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node interleaves items from multiple streams by taking one item from each stream
//! in round-robin order. When a stream ends, it is skipped and the interleaving continues
//! with the remaining streams. The interleaving stops when all streams have ended.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Type alias for a pinned stream of items
type PinnedItemStream =
  Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>;

/// A node that interleaves items from multiple streams.
///
/// The node receives items from multiple input streams (in_0, in_1, ..., in_n)
/// and interleaves them by taking one item from each stream in round-robin order.
/// When a stream ends, it is skipped and interleaving continues with remaining streams.
pub struct InterleaveNode {
  pub(crate) base: BaseNode,
}

impl InterleaveNode {
  /// Creates a new InterleaveNode with the given name and number of input streams.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  /// * `num_inputs` - The number of input streams to interleave (creates in_0, in_1, ..., in_n-1)
  ///
  /// # Returns
  ///
  /// A new `InterleaveNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::InterleaveNode;
  ///
  /// let node = InterleaveNode::new("interleave".to_string(), 2);
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
impl Node for InterleaveNode {
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
      let mut input_streams: Vec<(usize, crate::node::InputStream)> = Vec::new();

      // Collect all input streams and their indices
      for (port_name, stream) in inputs {
        if port_name.starts_with("in_")
          && let Ok(index) = port_name[3..].parse::<usize>()
        {
          input_streams.push((index, stream));
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

      // Process streams by interleaving them
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        // Convert streams to a vector of pinned streams
        let mut streams: Vec<PinnedItemStream> = input_streams
          .into_iter()
          .map(|(_, stream)| Box::pin(stream) as PinnedItemStream)
          .collect();

        // Track which streams are still active
        let mut active_streams: Vec<bool> = vec![true; streams.len()];
        let mut active_count = streams.len();

        // Interleave streams in round-robin order
        while active_count > 0 {
          for (idx, stream) in streams.iter_mut().enumerate() {
            if !active_streams[idx] {
              continue; // Skip ended streams
            }

            match stream.next().await {
              Some(item) => {
                // Forward item
                let _ = out_tx_clone.send(item).await;
              }
              None => {
                // Stream ended, mark as inactive
                active_streams[idx] = false;
                active_count -= 1;
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
