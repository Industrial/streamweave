//! # Group By Node
#![allow(clippy::type_complexity)]
//!
//! A reduction node that groups items from a stream by a key extracted using a configurable key function.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to group
//! - **Input**: `"key_function"` - Receives the key extraction function configuration
//! - **Output**: `"out"` - Sends the grouped results as a HashMap
//! - **Output**: `"error"` - Sends errors that occur during grouping
//!
//! ## Behavior
//!
//! The node groups items from the input stream by a key extracted using the key function.
//! It processes all items in the stream and outputs a single HashMap where:
//! - Keys are the extracted key strings
//! - Values are arrays of items that share the same key
//!   When the stream ends, it outputs the final grouped HashMap.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous key extraction functions used by GroupByNode.
///
/// Implementations of this trait define how to extract a key from an item.
/// The function receives an item and returns the key as a String.
#[async_trait]
pub trait GroupByKeyFunction: Send + Sync {
  /// Extracts a key from an item.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value to extract a key from
  ///
  /// # Returns
  ///
  /// `Ok(key_string)` if key extraction succeeds, or `Err(error_message)` if it fails.
  async fn extract_key(&self, value: Arc<dyn Any + Send + Sync>) -> Result<String, String>;
}

/// Configuration for GroupByNode that defines the key extraction operation.
///
/// Uses Arc for zero-copy sharing.
pub type GroupByConfig = Arc<dyn GroupByKeyFunction>;

/// Wrapper type to send GroupByConfig through streams.
///
/// Since we can't directly downcast `Arc<dyn Any>` to `Arc<dyn GroupByKeyFunction>`,
/// we wrap GroupByConfig in this struct so we can recover it from the stream.
pub struct GroupByConfigWrapper(pub GroupByConfig);

impl GroupByConfigWrapper {
  /// Creates a new wrapper from a GroupByConfig.
  pub fn new(config: GroupByConfig) -> Self {
    Self(config)
  }
}

/// Wrapper type that implements GroupByKeyFunction for async closures.
struct GroupByKeyFunctionWrapper<F> {
  function: F,
}

#[async_trait]
impl<F> GroupByKeyFunction for GroupByKeyFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
    + Send
    + Sync,
{
  async fn extract_key(&self, value: Arc<dyn Any + Send + Sync>) -> Result<String, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a GroupByConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::reduction::{GroupByConfig, group_by_config};
///
/// // Create a config that extracts a "category" property from objects
/// let config: GroupByConfig = group_by_config(|value| async move {
///     if let Ok(arc_map) = value.downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>() {
///         if let Some(category) = arc_map.get("category") {
///             if let Ok(cat_str) = category.clone().downcast::<String>() {
///                 Ok(cat_str.to_string())
///             } else {
///                 Err("Category is not a string".to_string())
///             }
///         } else {
///             Err("No category property".to_string())
///         }
///     } else {
///         Err("Expected HashMap".to_string())
///     }
/// });
/// ```
pub fn group_by_config<F, Fut>(function: F) -> GroupByConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<String, String>> + Send + 'static,
{
  Arc::new(GroupByKeyFunctionWrapper {
    function: move |v| {
      Box::pin(function(v))
        as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
    },
  })
}

/// Enum to tag input ports
#[allow(dead_code)]
enum InputPort {
  Config,
  In,
  KeyFunction,
}

/// A node that groups items from a stream by a key extracted using a configurable key function.
///
/// The key function is defined by configuration received on the "key_function" port.
/// Each item from the "in" port has its key extracted, and items are grouped by key.
/// When the stream ends, a HashMap is sent to the "out" port where:
/// - Keys are the extracted key strings
/// - Values are arrays of items that share the same key
pub struct GroupByNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl GroupByNode {
  /// Creates a new GroupByNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::reduction::GroupByNode;
  ///
  /// let node = GroupByNode::new("group_by".to_string());
  /// // Creates ports: configuration, in, key_function → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "key_function".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for GroupByNode {
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
      let key_function_stream = inputs
        .remove("key_function")
        .ok_or("Missing 'key_function' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let key_function_stream = key_function_stream.map(|item| (InputPort::KeyFunction, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(key_function_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged_stream = merged_stream;
        let mut data_buffer: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
        let mut key_function: Option<GroupByConfig> = None;

        // First, collect key function and buffer data items
        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Configuration port is unused for now
            }
            InputPort::KeyFunction => {
              // Set key function
              if let Ok(wrapper) = Arc::downcast::<GroupByConfigWrapper>(item.clone()) {
                key_function = Some(wrapper.0.clone());
              } else {
                let error_msg = format!(
                  "Invalid key function type: {} (expected GroupByConfigWrapper)",
                  std::any::type_name_of_val(&*item)
                );
                let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                return;
              }
            }
            InputPort::In => {
              // Buffer data items
              data_buffer.push(item);
            }
          }
        }

        // Now process buffered data items if we have key function
        if let Some(func) = key_function {
          let mut groups: HashMap<String, Vec<Arc<dyn Any + Send + Sync>>> = HashMap::new();

          for item in data_buffer {
            // Extract key and group item
            match func.extract_key(item.clone()).await {
              Ok(key) => {
                groups.entry(key).or_default().push(item);
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Continue processing other items even if one fails
              }
            }
          }

          // Convert groups HashMap to output format
          // Output is HashMap<String, Vec<Arc<dyn Any + Send + Sync>>>
          let grouped_result: HashMap<String, Arc<dyn Any + Send + Sync>> = groups
            .into_iter()
            .map(|(key, items)| {
              // Convert Vec<Arc<dyn Any + Send + Sync>> to Arc<dyn Any + Send + Sync>
              let items_arc = Arc::new(items) as Arc<dyn Any + Send + Sync>;
              (key, items_arc)
            })
            .collect();

          let result_arc = Arc::new(grouped_result) as Arc<dyn Any + Send + Sync>;
          let _ = out_tx_clone.send(result_arc).await;
        } else {
          // No key function provided
          let error_msg = "No key function provided for grouping".to_string();
          let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
          let _ = error_tx_clone.send(error_arc).await;
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
