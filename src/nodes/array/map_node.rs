//! # Array Map Node
//!
//! A transform node that applies a transformation function to each element in an array.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives array value (Vec<Arc<dyn Any + Send + Sync>>)
//! - **Input**: `"function"` - Receives map transformation function (MapFunction)
//! - **Output**: `"out"` - Sends the mapped array
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node applies a transformation function to each element in an array and returns the result. It supports:
//! - Mapping based on a transformation function that transforms each element
//! - Preserving element order
//! - Error handling: Non-array inputs or transformation errors result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::array::common::array_map;
use crate::nodes::common::{BaseNode, process_configurable_node};
use crate::nodes::map_node::MapConfig;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

/// A node that applies a transformation function to each element in an array.
///
/// The node receives an array value on the "in" port and a transformation function on the "function" port,
/// then outputs the mapped array to the "out" port.
pub struct ArrayMapNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl ArrayMapNode {
  /// Creates a new ArrayMapNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::array::ArrayMapNode;
  ///
  /// let node = ArrayMapNode::new("map".to_string());
  /// // Creates ports: configuration, in, function → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "function".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for ArrayMapNode {
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
      let function_stream = inputs
        .remove("function")
        .ok_or("Missing 'function' input")?;

      // Process using unified helper with zero-copy guarantee
      let (out_rx, error_rx) = process_configurable_node(
        function_stream,
        in_stream,
        Arc::new(Mutex::new(None::<Arc<MapConfig>>)),
        |item: Arc<dyn Any + Send + Sync>, cfg: &Arc<MapConfig>| {
          let cfg = cfg.clone();
          async move {
            // Zero-copy: cfg.apply receives item by value (Arc), only refcount is incremented
            array_map(&item, &cfg).await.map(Some)
          }
        },
      );

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
