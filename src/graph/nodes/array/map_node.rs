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

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::MapConfig;
use crate::graph::nodes::array::common::array_map;
use crate::graph::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that applies a transformation function to each element in an array.
///
/// The node receives an array value on the "in" port and a transformation function on the "function" port,
/// then outputs the mapped array to the "out" port.
pub struct ArrayMapNode {
  pub(crate) base: BaseNode,
  current_map_fn: Arc<Mutex<Option<Arc<MapConfig>>>>,
}

impl ArrayMapNode {
  /// Creates a new ArrayMapNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::array::ArrayMapNode;
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
      current_map_fn: Arc::new(Mutex::new(None::<Arc<MapConfig>>)),
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
    let map_fn_state = Arc::clone(&self.current_map_fn);

    Box::pin(async move {
      // Extract input streams (configuration port is present but unused for now)
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let function_stream = inputs
        .remove("function")
        .ok_or("Missing 'function' input")?;

      // Tag streams to distinguish config from data
      let function_stream = function_stream.map(|item| (MessageType::Config, item));
      let in_stream = in_stream.map(|item| (MessageType::Data, item));

      // Merge streams
      let merged_stream = stream::select(function_stream, in_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();
      let map_fn_state_clone = Arc::clone(&map_fn_state);

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut pending_array: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Try to downcast function to MapConfig
              if let Ok(map_fn_arc) = item.downcast::<MapConfig>() {
                let mut map_fn = map_fn_state_clone.lock().await;
                *map_fn = Some(map_fn_arc);
                // Process pending array if we have one
                if let Some(array) = pending_array.take() {
                  let map_fn_opt = {
                    let map_fn = map_fn_state_clone.lock().await;
                    map_fn.clone()
                  };
                  if let Some(map_fn) = map_fn_opt {
                    match array_map(&array, &map_fn).await {
                      Ok(result) => {
                        let _ = out_tx_clone.send(result).await;
                      }
                      Err(e) => {
                        let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                        let _ = error_tx_clone.send(error_arc).await;
                      }
                    }
                  }
                }
              } else {
                // Invalid function type - send error
                let error_arc = Arc::new("Invalid function type: expected MapConfig".to_string())
                  as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              // Get the current map function
              let map_fn_opt = {
                let map_fn = map_fn_state_clone.lock().await;
                map_fn.clone()
              };

              if let Some(map_fn) = map_fn_opt {
                match array_map(&item, &map_fn).await {
                  Ok(result) => {
                    let _ = out_tx_clone.send(result).await;
                  }
                  Err(e) => {
                    let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
              } else {
                // No function set yet - store array for later processing
                pending_array = Some(item);
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
