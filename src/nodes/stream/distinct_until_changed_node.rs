//! # Distinct Until Changed Node
//!
//! A transform node that filters a stream to emit items only when they differ
//! from the previously emitted item.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to filter for changes
//! - **Output**: `"out"` - Sends items that differ from the previous emitted item
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node tracks the last emitted item and only emits new items that differ from it.
//! This is useful for filtering out consecutive duplicates while allowing the same value
//! to be emitted later if intervening different values were emitted.
//!
//! Items are compared by attempting to downcast to common types (integers, strings, etc.)
//! and comparing their values. If downcast fails, falls back to Arc pointer comparison.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to compare two Arc<dyn Any + Send + Sync> values
/// Returns true if they are considered equal (same value)
fn values_equal(a: &Arc<dyn Any + Send + Sync>, b: &Arc<dyn Any + Send + Sync>) -> bool {
  // Try value-based comparison for common types
  if let (Ok(a_val), Ok(b_val)) = (a.clone().downcast::<i32>(), b.clone().downcast::<i32>()) {
    return *a_val == *b_val;
  }
  if let (Ok(a_val), Ok(b_val)) = (a.clone().downcast::<i64>(), b.clone().downcast::<i64>()) {
    return *a_val == *b_val;
  }
  if let (Ok(a_val), Ok(b_val)) = (a.clone().downcast::<u32>(), b.clone().downcast::<u32>()) {
    return *a_val == *b_val;
  }
  if let (Ok(a_val), Ok(b_val)) = (a.clone().downcast::<u64>(), b.clone().downcast::<u64>()) {
    return *a_val == *b_val;
  }
  if let (Ok(a_val), Ok(b_val)) = (
    a.clone().downcast::<String>(),
    b.clone().downcast::<String>(),
  ) {
    return *a_val == *b_val;
  }
  if let (Ok(a_val), Ok(b_val)) = (a.clone().downcast::<bool>(), b.clone().downcast::<bool>()) {
    return *a_val == *b_val;
  }

  // Fall back to pointer comparison for other types
  Arc::ptr_eq(a, b)
}

/// A node that filters a stream to emit items only when they differ
/// from the previously emitted item.
///
/// The node tracks the last emitted value and only emits new values
/// that are different from the last one.
pub struct DistinctUntilChangedNode {
  pub(crate) base: BaseNode,
}

impl DistinctUntilChangedNode {
  /// Creates a new DistinctUntilChangedNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `DistinctUntilChangedNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::DistinctUntilChangedNode;
  ///
  /// let node = DistinctUntilChangedNode::new("distinct_until_changed".to_string());
  /// // Creates ports: configuration, in → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for DistinctUntilChangedNode {
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
      // Extract input streams
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the input stream
      let out_tx_clone = out_tx.clone();
      let _error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut last_emitted: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some(item) = in_stream.next().await {
          // Check if this item differs from the last emitted item
          let should_emit = if let Some(ref last) = last_emitted {
            !values_equal(last, &item)
          } else {
            // First item should always be emitted
            true
          };

          if should_emit {
            // Emit the item and update last_emitted
            let _ = out_tx_clone.send(item.clone()).await;
            last_emitted = Some(item);
          }
          // Items that are the same as the last emitted are silently ignored
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
