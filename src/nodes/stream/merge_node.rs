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
//!
//! For deterministic order, use [`MergeNode::new_deterministic`], which merges
//! in round-robin by port index (port 0, then 1, then 2, ...).

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Type alias for a pinned stream of items
type PinnedItemStream =
  Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>;

/// A node that merges multiple streams together.
///
/// The node receives items from multiple input streams (in_0, in_1, ..., in_n)
/// and merges them by forwarding items from any stream as they arrive.
/// The order is non-deterministic and depends on which stream has items available.
/// The merge continues until all streams have ended.
///
/// Use [`MergeNode::new_deterministic`] for reproducible order (round-robin by port index).
pub struct MergeNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// When true, merge in round-robin by port index for deterministic order.
  deterministic: bool,
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
  /// use streamweave::nodes::stream::MergeNode;
  ///
  /// let node = MergeNode::new("merge".to_string(), 2);
  /// // Creates ports: configuration, in_0, in_1 → out, error
  /// ```
  pub fn new(name: String, num_inputs: usize) -> Self {
    Self::new_with_deterministic(name, num_inputs, false)
  }

  /// Creates a MergeNode that merges in round-robin by port index for deterministic order.
  ///
  /// Items are merged in order: port 0, then port 1, then port 2, etc. (round-robin).
  /// This gives reproducible output for the same inputs, suitable for use with
  /// [`ExecutionMode::Deterministic`](crate::graph::ExecutionMode::Deterministic).
  pub fn new_deterministic(name: String, num_inputs: usize) -> Self {
    Self::new_with_deterministic(name, num_inputs, true)
  }

  /// Builds a merge node with the given name, number of inputs, and deterministic flag.
  fn new_with_deterministic(name: String, num_inputs: usize, deterministic: bool) -> Self {
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
      deterministic,
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
    let deterministic = self.deterministic;
    Box::pin(async move {
      // Extract configuration stream
      let _config_stream = inputs.remove("configuration");

      // Extract all input streams (in_0, in_1, ..., in_n) with port index
      let mut input_streams: Vec<(usize, crate::node::InputStream)> = Vec::new();
      for (port_name, stream) in inputs {
        if port_name.starts_with("in_")
          && let Ok(index) = port_name[3..].parse::<usize>()
        {
          input_streams.push((index, stream));
        }
      }
      input_streams.sort_by_key(|(idx, _)| *idx);

      if input_streams.is_empty() {
        return Err("No input streams found (expected in_0, in_1, ...)".into());
      }

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone();

      if deterministic {
        // Deterministic merge: round-robin by port index (port 0, 1, 2, ...)
        let streams: Vec<PinnedItemStream> = input_streams
          .into_iter()
          .map(|(_, s)| Box::pin(s) as PinnedItemStream)
          .collect();
        tokio::spawn(async move {
          let mut streams = streams;
          let mut active = vec![true; streams.len()];
          let mut active_count = streams.len();
          while active_count > 0 {
            for (idx, stream) in streams.iter_mut().enumerate() {
              if !active[idx] {
                continue;
              }
              match stream.next().await {
                Some(item) => {
                  let _ = out_tx_clone.send(item).await;
                }
                None => {
                  active[idx] = false;
                  active_count -= 1;
                }
              }
            }
          }
        });
      } else {
        // Non-deterministic merge: select_all (items as they arrive)
        let tagged_streams: Vec<_> = input_streams
          .into_iter()
          .map(|(_, s)| Box::pin(s) as PinnedItemStream)
          .collect();
        let merged_stream = stream::select_all(tagged_streams);
        tokio::spawn(async move {
          let mut merged = merged_stream;
          while let Some(item) = merged.next().await {
            let _ = out_tx_clone.send(item).await;
          }
        });
      }

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
