//! # Is String Node
//!
//! A transform node that checks if a value is a string.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives any value
//! - **Output**: `"out"` - Sends boolean (true if string, false otherwise)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node checks if the input value is a String type.
//! It returns true for: String
//! It returns false for: numbers, bool, Vec, HashMap, and other non-string types.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Checks if a value is a String type.
///
/// Returns true if the value can be downcast to String.
fn is_string(v: &Arc<dyn Any + Send + Sync>) -> bool {
  v.clone().downcast::<String>().is_ok()
}

/// A node that checks if a value is a string.
///
/// The node receives any value on the "in" port and outputs
/// a boolean (true if string, false otherwise) to the "out" port.
pub struct IsStringNode {
  pub(crate) base: BaseNode,
}

impl IsStringNode {
  /// Creates a new IsStringNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `IsStringNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::type_ops::IsStringNode;
  ///
  /// let node = IsStringNode::new("is_string".to_string());
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
impl Node for IsStringNode {
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
      let _error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut stream = in_stream;
        while let Some(item) = stream.next().await {
          let is_str = is_string(&item);
          let result = Arc::new(is_str) as Arc<dyn Any + Send + Sync>;
          let _ = out_tx_clone.send(result).await;
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

