//! # To Boolean Node
//!
//! A transform node that converts a value to a boolean.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives any value
//! - **Output**: `"out"` - Sends the value converted to boolean
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., unsupported types)
//!
//! ## Behavior
//!
//! The node converts the input value to a boolean representation.
//! It supports:
//! - Boolean: Returns as-is
//! - Numbers: 0 or 0.0 -> false, non-zero -> true
//! - String: Empty string -> false, non-empty -> true
//! - Arrays: Empty array -> false, non-empty -> true
//! - Objects: Empty object -> false, non-empty -> true
//! - Error handling: Unsupported types result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Converts a value to a boolean.
///
/// This function attempts to convert various types to boolean:
/// - Boolean: Returns as-is
/// - Numbers: 0 or 0.0 -> false, non-zero -> true
/// - String: Empty string -> false, non-empty -> true
/// - Arrays: Empty array -> false, non-empty -> true
/// - Objects: Empty object -> false, non-empty -> true
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (bool) or an error string.
fn to_boolean(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try boolean (already a boolean)
  if let Ok(arc_bool) = v.clone().downcast::<bool>() {
    return Ok(arc_bool);
  }

  // Try numbers (0 or 0.0 -> false, non-zero -> true)
  if let Ok(arc_i32) = v.clone().downcast::<i32>() {
    return Ok(Arc::new(*arc_i32 != 0) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_i64) = v.clone().downcast::<i64>() {
    return Ok(Arc::new(*arc_i64 != 0) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u32) = v.clone().downcast::<u32>() {
    return Ok(Arc::new(*arc_u32 != 0) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u64) = v.clone().downcast::<u64>() {
    return Ok(Arc::new(*arc_u64 != 0) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f32) = v.clone().downcast::<f32>() {
    return Ok(Arc::new(*arc_f32 != 0.0) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f64) = v.clone().downcast::<f64>() {
    return Ok(Arc::new(*arc_f64 != 0.0) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_usize) = v.clone().downcast::<usize>() {
    return Ok(Arc::new(*arc_usize != 0) as Arc<dyn Any + Send + Sync>);
  }

  // Try string (empty -> false, non-empty -> true)
  if let Ok(arc_str) = v.clone().downcast::<String>() {
    return Ok(Arc::new(!arc_str.is_empty()) as Arc<dyn Any + Send + Sync>);
  }

  // Try array (empty -> false, non-empty -> true)
  if let Ok(arc_vec) = v.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
    return Ok(Arc::new(!arc_vec.is_empty()) as Arc<dyn Any + Send + Sync>);
  }

  // Try object (empty -> false, non-empty -> true)
  if let Ok(arc_map) = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    return Ok(Arc::new(!arc_map.is_empty()) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for boolean conversion: {}",
    std::any::type_name_of_val(&**v)
  ))
}

/// A node that converts a value to a boolean.
///
/// The node receives any value on the "in" port and outputs
/// its boolean representation to the "out" port.
pub struct ToBooleanNode {
  pub(crate) base: BaseNode,
}

impl ToBooleanNode {
  /// Creates a new ToBooleanNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `ToBooleanNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::type_ops::ToBooleanNode;
  ///
  /// let node = ToBooleanNode::new("to_boolean".to_string());
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
impl Node for ToBooleanNode {
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

      // Create output streams
      let (out_tx, out_rx) = mpsc::channel(10);
      let (error_tx, error_rx) = mpsc::channel(10);

      // Process the input stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut stream = in_stream;
        while let Some(item) = stream.next().await {
          match to_boolean(&item) {
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
