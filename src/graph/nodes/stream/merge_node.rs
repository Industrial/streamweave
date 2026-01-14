//! # Merge Node
//!
//! A transform node that merges multiple streams together.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in_0"`, `"in_1"`, ..., `"in_n"` - Receives data items from multiple streams
//! - **Output**: `"out"` - Sends merged items (items from any stream as they arrive)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node merges multiple streams together by forwarding items from any stream
//! as they arrive. The order is non-deterministic and depends on which stream
//! has items available. The merge continues until all streams have ended.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that merges multiple streams together.
///
/// The node receives items from multiple input streams (in_0, in_1, ..., in_n)
/// and merges them by forwarding items from any stream as they arrive.
/// The order is non-deterministic and depends on which stream has items available.
/// The merge continues until all streams have ended.
pub struct MergeNode {
  pub(crate) base: BaseNode,
}

impl MergeNode {
  /// Creates a new MergeNode with the given name and number of input streams.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  /// * `num_inputs` - The number of input streams to merge (creates in_0, in_1, ..., in_n-1)
  ///
  /// # Returns
  ///
  /// A new `MergeNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::stream::MergeNode;
  ///
  /// let node = MergeNode::new("merge".to_string(), 2);
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
impl Node for MergeNode {
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
      let mut tagged_streams = Vec::new();

      // Collect all input streams and tag them
      for (port_name, stream) in inputs {
        if port_name.starts_with("in_") {
          let tagged = stream.map(move |item| item);
          tagged_streams.push(Box::pin(tagged)
            as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>);
        }
      }

      if tagged_streams.is_empty() {
        return Err("No input streams found (expected in_0, in_1, ...)".into());
      }

      // Merge streams using select_all - items arrive as they're available
      let merged_stream = stream::select_all(tagged_streams);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process merged stream
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        while let Some(item) = merged.next().await {
          // Forward item from any stream
          let _ = out_tx_clone.send(item).await;
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

