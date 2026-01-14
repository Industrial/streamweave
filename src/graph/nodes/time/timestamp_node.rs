//! # Timestamp Node
//!
//! A transform node that adds timestamps to items from the input stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to timestamp
//! - **Output**: `"out"` - Sends items with timestamps added
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node adds a timestamp to each item from the input stream. The timestamp is added as:
//! - If the item is an object (HashMap), a "timestamp" property is added
//! - If the item is not an object, it is wrapped in an object with "value" (the original item) and "timestamp" properties
//! The timestamp is represented as i64 milliseconds since Unix epoch.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Adds a timestamp to an item.
///
/// If the item is an object (HashMap), adds a "timestamp" property.
/// If the item is not an object, wraps it in an object with "value" and "timestamp" properties.
fn add_timestamp(item: Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Generate timestamp (milliseconds since epoch)
  let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;
  let timestamp_arc = Arc::new(timestamp) as Arc<dyn Any + Send + Sync>;

  // Try to downcast to HashMap (object)
  if let Ok(arc_map) = item.clone().downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>() {
    // Item is an object - add timestamp property
    let mut new_map = (*arc_map).clone();
    new_map.insert("timestamp".to_string(), timestamp_arc);
    Ok(Arc::new(new_map) as Arc<dyn Any + Send + Sync>)
  } else {
    // Item is not an object - wrap it in an object
    let mut wrapped = HashMap::new();
    wrapped.insert("value".to_string(), item);
    wrapped.insert("timestamp".to_string(), timestamp_arc);
    Ok(Arc::new(wrapped) as Arc<dyn Any + Send + Sync>)
  }
}

/// A node that adds timestamps to items from the input stream.
///
/// Each item from the "in" port has a timestamp added:
/// - If the item is an object (HashMap), a "timestamp" property is added
/// - If the item is not an object, it is wrapped in an object with "value" and "timestamp" properties
/// The timestamp is represented as i64 milliseconds since Unix epoch.
pub struct TimestampNode {
  pub(crate) base: BaseNode,
}

impl TimestampNode {
  /// Creates a new TimestampNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::time::TimestampNode;
  ///
  /// let node = TimestampNode::new("timestamp".to_string());
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
impl Node for TimestampNode {
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
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        while let Some(item) = in_stream.next().await {
          match add_timestamp(item) {
            Ok(result) => {
              let _ = out_tx_clone.send(result).await;
            }
            Err(e) => {
              let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
              let _ = error_tx_clone.send(error_arc).await;
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

