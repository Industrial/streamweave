//! # Distinct Node
//!
//! A transform node that filters a stream to emit only distinct (unique) items.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to filter for distinctness
//! - **Output**: `"out"` - Sends items that haven't been seen before
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node maintains an internal set of items that have been seen.
//! For each item received on the "in" port:
//! - If the item has not been seen before, it is emitted on the "out" port
//! - The item is then added to the set of seen items
//! - Duplicate items are silently ignored
//!
//! Items are compared by attempting to downcast to common types (integers, strings, etc.)
//! and comparing their values. If downcast fails, falls back to Arc pointer comparison.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Wrapper to make Arc<dyn Any + Send + Sync> comparable for the HashSet
/// Attempts value-based comparison for common types, falls back to pointer comparison
#[derive(Debug, Clone)]
struct ArcWrapper(Arc<dyn Any + Send + Sync>);

impl PartialEq for ArcWrapper {
  fn eq(&self, other: &Self) -> bool {
    // Try value-based comparison for common types
    if let (Ok(a), Ok(b)) = (
      self.0.clone().downcast::<i32>(),
      other.0.clone().downcast::<i32>(),
    ) {
      return *a == *b;
    }
    if let (Ok(a), Ok(b)) = (
      self.0.clone().downcast::<i64>(),
      other.0.clone().downcast::<i64>(),
    ) {
      return *a == *b;
    }
    if let (Ok(a), Ok(b)) = (
      self.0.clone().downcast::<u32>(),
      other.0.clone().downcast::<u32>(),
    ) {
      return *a == *b;
    }
    if let (Ok(a), Ok(b)) = (
      self.0.clone().downcast::<u64>(),
      other.0.clone().downcast::<u64>(),
    ) {
      return *a == *b;
    }
    if let (Ok(a), Ok(b)) = (
      self.0.clone().downcast::<String>(),
      other.0.clone().downcast::<String>(),
    ) {
      return *a == *b;
    }
    if let (Ok(a), Ok(b)) = (
      self.0.clone().downcast::<bool>(),
      other.0.clone().downcast::<bool>(),
    ) {
      return *a == *b;
    }

    // Fall back to pointer comparison for other types
    Arc::ptr_eq(&self.0, &other.0)
  }
}

impl Eq for ArcWrapper {}

impl Hash for ArcWrapper {
  fn hash<H: Hasher>(&self, state: &mut H) {
    // Try value-based hashing for common types
    if let Ok(val) = self.0.clone().downcast::<i32>() {
      return (*val).hash(state);
    }
    if let Ok(val) = self.0.clone().downcast::<i64>() {
      return (*val).hash(state);
    }
    if let Ok(val) = self.0.clone().downcast::<u32>() {
      return (*val).hash(state);
    }
    if let Ok(val) = self.0.clone().downcast::<u64>() {
      return (*val).hash(state);
    }
    if let Ok(val) = self.0.clone().downcast::<String>() {
      return (*val).hash(state);
    }
    if let Ok(val) = self.0.clone().downcast::<bool>() {
      return (*val).hash(state);
    }

    // Fall back to pointer hashing for other types
    (Arc::as_ptr(&self.0) as *const () as usize).hash(state);
  }
}

/// A node that filters a stream to emit only distinct (unique) items.
///
/// The node maintains a set of items that have been seen and only emits
/// items that haven't been encountered before.
pub struct DistinctNode {
  pub(crate) base: BaseNode,
}

impl DistinctNode {
  /// Creates a new DistinctNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `DistinctNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::DistinctNode;
  ///
  /// let node = DistinctNode::new("distinct".to_string());
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
impl Node for DistinctNode {
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
        let mut seen_items: HashSet<ArcWrapper> = HashSet::new();
        let mut in_stream = in_stream;

        while let Some(item) = in_stream.next().await {
          let wrapper = ArcWrapper(item.clone());

          // Check if we've seen this item before
          if !seen_items.contains(&wrapper) {
            // This is a new item, emit it and add to seen set
            let _ = out_tx_clone.send(item).await;
            seen_items.insert(wrapper);
          }
          // Duplicate items are silently ignored
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
