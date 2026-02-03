//! # Array Filter Node
//!
//! A transform node that filters elements in an array based on a predicate function.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives array value (Vec<Arc<dyn Any + Send + Sync>>)
//! - **Input**: `"predicate"` - Receives filter predicate function (FilterFunction)
//! - **Output**: `"out"` - Sends the filtered array
//! - **Output**: `"error"` - Sends errors that occur during processing (e.g., type mismatch)
//!
//! ## Behavior
//!
//! The node filters elements in an array based on a predicate function and returns the result. It supports:
//! - Filtering based on a predicate function that evaluates each element
//! - Preserving element references (zero-copy for kept elements)
//! - Error handling: Non-array inputs or predicate errors result in errors sent to the error port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::array::common::array_filter;
use crate::nodes::common::{BaseNode, MessageType};
use crate::nodes::filter_node::FilterConfig;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that filters elements in an array based on a predicate function.
///
/// The node receives an array value on the "in" port and a predicate function configuration
/// on the "predicate" port, then outputs the filtered array to the "out" port.
pub struct ArrayFilterNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current predicate function configuration state.
  current_predicate: Arc<Mutex<Option<Arc<FilterConfig>>>>,
}

impl ArrayFilterNode {
  /// Creates a new ArrayFilterNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::array::ArrayFilterNode;
  ///
  /// let node = ArrayFilterNode::new("filter".to_string());
  /// // Creates ports: configuration, in, predicate → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "predicate".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_predicate: Arc::new(Mutex::new(None::<Arc<FilterConfig>>)),
    }
  }
}

#[async_trait]
impl Node for ArrayFilterNode {
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
    let predicate_state: Arc<tokio::sync::Mutex<Option<Arc<FilterConfig>>>> =
      Arc::clone(&self.current_predicate);

    Box::pin(async move {
      // Extract input streams (configuration port is present but unused for now)
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let predicate_stream = inputs
        .remove("predicate")
        .ok_or("Missing 'predicate' input")?;

      // Tag streams to distinguish config from data
      let predicate_stream = predicate_stream.map(|item| (MessageType::Config, item));
      let in_stream = in_stream.map(|item| (MessageType::Data, item));

      // Merge streams
      let merged_stream = stream::select(predicate_stream, in_stream);

      // Create output streams
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();
      let predicate_state_clone = Arc::clone(&predicate_state);

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut pending_array: Option<Arc<dyn Any + Send + Sync>> = None;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Try to downcast predicate to FilterConfig
              if let Ok(predicate_arc) = item.downcast::<FilterConfig>() {
                let mut pred = predicate_state_clone.lock().await;
                *pred = Some(predicate_arc);
                // Process pending array if we have one
                if let Some(array) = pending_array.take() {
                  let predicate_opt = {
                    let pred = predicate_state_clone.lock().await;
                    pred.clone()
                  };
                  if let Some(predicate) = predicate_opt {
                    match array_filter(&array, &predicate).await {
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
                // Invalid predicate type - send error
                let error_arc =
                  Arc::new("Invalid predicate type: expected FilterConfig".to_string())
                    as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              // Get the current predicate
              let predicate_opt = {
                let pred = predicate_state_clone.lock().await;
                pred.clone()
              };

              if let Some(predicate) = predicate_opt {
                match array_filter(&item, &predicate).await {
                  Ok(result) => {
                    let _ = out_tx_clone.send(result).await;
                  }
                  Err(e) => {
                    let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
              } else {
                // No predicate set yet - store array for later processing
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
