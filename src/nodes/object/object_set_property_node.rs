//! # Object Set Property Node
//!
//! A transform node that sets a property value in an object (HashMap).
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives object value (HashMap<String, Arc<dyn Any + Send + Sync>>)
//! - **Input**: `"key"` - Receives key value (String)
//! - **Input**: `"value"` - Receives value to set (any type)
//! - **Output**: `"out"` - Sends the object with the property set
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node sets a property value in an object and returns a new object. It supports:
//! - Setting values in HashMap<String, Arc<dyn Any + Send + Sync>> objects
//! - Key must be a String
//! - Creates a new object with the property set (does not modify the original)
//! - If the key already exists, it is overwritten
//! - Error handling: Non-object inputs and invalid key types result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::object::common::object_set_property;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Enum to tag input ports
enum InputPort {
  In,
  Key,
  Value,
}

/// A node that sets a property value in an object.
///
/// The node receives an object value on the "in" port, a key on the "key" port,
/// and a value on the "value" port, then outputs the object with the property set to the "out" port.
pub struct ObjectSetPropertyNode {
  pub(crate) base: BaseNode,
}

impl ObjectSetPropertyNode {
  /// Creates a new ObjectSetPropertyNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::object::ObjectSetPropertyNode;
  ///
  /// let node = ObjectSetPropertyNode::new("set_property".to_string());
  /// // Creates ports: configuration, in, key, value → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "key".to_string(),
          "value".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[allow(clippy::type_complexity)]
#[async_trait]
impl Node for ObjectSetPropertyNode {
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
      let key_stream = inputs.remove("key").ok_or("Missing 'key' input")?;
      let value_stream = inputs.remove("value").ok_or("Missing 'value' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let key_stream = key_stream.map(|item| (InputPort::Key, item));
      let value_stream = value_stream.map(|item| (InputPort::Value, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(key_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(value_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream with buffering
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut in_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut key_buffer: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut value_buffer: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::In => {
              in_buffer = Some(item);
            }
            InputPort::Key => {
              key_buffer = Some(item);
            }
            InputPort::Value => {
              value_buffer = Some(item);
            }
          }

          // Process when all inputs are available
          if let (Some(v), Some(key), Some(value)) = (
            in_buffer.as_ref(),
            key_buffer.as_ref(),
            value_buffer.as_ref(),
          ) {
            match object_set_property(v, key, value) {
              Ok(result) => {
                let _ = out_tx_clone.send(result).await;
                // Clear buffers after processing
                in_buffer = None;
                key_buffer = None;
                value_buffer = None;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Clear buffers after error
                in_buffer = None;
                key_buffer = None;
                value_buffer = None;
              }
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
