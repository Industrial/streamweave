//! # To Number Node
//!
//! A transform node that converts a value to a number.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives any value
//! - **Output**: `"out"` - Sends the value converted to number (f64)
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., invalid conversions)
//!
//! ## Behavior
//!
//! The node converts the input value to a numeric representation (f64).
//! It supports:
//! - Numbers: i32, i64, u32, u64, f32, f64, usize (converted to f64)
//! - String: Parsed to f64 (returns error if parsing fails)
//! - Boolean: true -> 1.0, false -> 0.0
//! - Error handling: Unsupported types or invalid string parsing result in errors sent to the error port

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Converts a value to a number (f64).
///
/// This function attempts to convert various types to f64:
/// - Numbers: i32, i64, u32, u64, f32, f64, usize (converted to f64)
/// - String: Parsed to f64 (returns error if parsing fails)
/// - Boolean: true -> 1.0, false -> 0.0
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (f64) or an error string.
fn to_number(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try numbers (convert all to f64)
  if let Ok(arc_i32) = v.clone().downcast::<i32>() {
    return Ok(Arc::new(*arc_i32 as f64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_i64) = v.clone().downcast::<i64>() {
    return Ok(Arc::new(*arc_i64 as f64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u32) = v.clone().downcast::<u32>() {
    return Ok(Arc::new(*arc_u32 as f64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u64) = v.clone().downcast::<u64>() {
    return Ok(Arc::new(*arc_u64 as f64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f32) = v.clone().downcast::<f32>() {
    return Ok(Arc::new(*arc_f32 as f64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f64) = v.clone().downcast::<f64>() {
    return Ok(arc_f64);
  }
  if let Ok(arc_usize) = v.clone().downcast::<usize>() {
    return Ok(Arc::new(*arc_usize as f64) as Arc<dyn Any + Send + Sync>);
  }

  // Try string (parse to f64)
  if let Ok(arc_str) = v.clone().downcast::<String>() {
    match arc_str.parse::<f64>() {
      Ok(num) => return Ok(Arc::new(num) as Arc<dyn Any + Send + Sync>),
      Err(e) => {
        return Err(format!(
          "Cannot parse string to number: \"{}\" ({})",
          arc_str, e
        ));
      }
    }
  }

  // Try boolean (true -> 1.0, false -> 0.0)
  if let Ok(arc_bool) = v.clone().downcast::<bool>() {
    return Ok(Arc::new(if *arc_bool { 1.0 } else { 0.0 }) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for number conversion: {}",
    std::any::type_name_of_val(&**v)
  ))
}

/// A node that converts a value to a number.
///
/// The node receives any value on the "in" port and outputs
/// its numeric representation (f64) to the "out" port.
pub struct ToNumberNode {
  pub(crate) base: BaseNode,
}

impl ToNumberNode {
  /// Creates a new ToNumberNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `ToNumberNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::type_ops::ToNumberNode;
  ///
  /// let node = ToNumberNode::new("to_number".to_string());
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
impl Node for ToNumberNode {
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
          match to_number(&item) {
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
