//! # To Array Node
//!
//! A transform node that converts a value to an array.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives any value
//! - **Output**: `"out"` - Sends the value converted to array (Vec<Arc<dyn Any + Send + Sync>>)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., unsupported types)
//!
//! ## Behavior
//!
//! The node converts the input value to an array representation.
//! It supports:
//! - Array: Returns as-is
//! - String: Converts to array of characters (each character as a String)
//! - Object: Converts to array of values
//! - Single values: Wraps in an array (numbers, booleans, etc.)
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

/// Converts a value to an array.
///
/// This function attempts to convert various types to array:
/// - Array: Returns as-is
/// - String: Converts to array of characters (each character as a String)
/// - Object: Converts to array of values
/// - Single values: Wraps in an array (numbers, booleans, etc.)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
fn to_array(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try array (already an array)
  if let Ok(_arc_vec) = v.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
    // For arrays, return as-is (already an array)
    return Ok(v.clone());
  }

  // Try string (convert to array of characters)
  if let Ok(arc_str) = v.clone().downcast::<String>() {
    let chars: Vec<Arc<dyn Any + Send + Sync>> = arc_str
      .chars()
      .map(|c| Arc::new(c.to_string()) as Arc<dyn Any + Send + Sync>)
      .collect();
    return Ok(Arc::new(chars) as Arc<dyn Any + Send + Sync>);
  }

  // Try object (convert to array of values)
  if let Ok(arc_map) = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    let values: Vec<Arc<dyn Any + Send + Sync>> = arc_map.values().cloned().collect();
    return Ok(Arc::new(values) as Arc<dyn Any + Send + Sync>);
  }

  // Try numbers (wrap in array)
  if let Ok(arc_i32) = v.clone().downcast::<i32>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_i32) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }
  if let Ok(arc_i64) = v.clone().downcast::<i64>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_i64) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }
  if let Ok(arc_u32) = v.clone().downcast::<u32>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_u32) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }
  if let Ok(arc_u64) = v.clone().downcast::<u64>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_u64) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }
  if let Ok(arc_f32) = v.clone().downcast::<f32>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_f32) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }
  if let Ok(arc_f64) = v.clone().downcast::<f64>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_f64) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }
  if let Ok(arc_usize) = v.clone().downcast::<usize>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_usize) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }

  // Try boolean (wrap in array)
  if let Ok(arc_bool) = v.clone().downcast::<bool>() {
    return Ok(
      Arc::new(vec![Arc::new(*arc_bool) as Arc<dyn Any + Send + Sync>])
        as Arc<dyn Any + Send + Sync>,
    );
  }

  Err(format!(
    "Unsupported type for array conversion: {}",
    std::any::type_name_of_val(&**v)
  ))
}

/// A node that converts a value to an array.
///
/// The node receives any value on the "in" port and outputs
/// its array representation to the "out" port.
pub struct ToArrayNode {
  pub(crate) base: BaseNode,
}

impl ToArrayNode {
  /// Creates a new ToArrayNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `ToArrayNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::type_ops::ToArrayNode;
  ///
  /// let node = ToArrayNode::new("to_array".to_string());
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
impl Node for ToArrayNode {
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
          match to_array(&item) {
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
