//! # Filter Map Node
//!
//! A transform node that applies a filter-map operation to a stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to filter and map
//! - **Input**: `"function"` - Receives the filter-map function configuration
//! - **Output**: `"out"` - Sends transformed items (Some results from the function)
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node applies a filter-map function to each item from the input stream.
//! The function returns `Option<T>` where:
//! - `Some(value)` - the item is transformed and emitted
//! - `None` - the item is filtered out (not emitted)
//!
//! Items that result in errors during processing are sent to the error port.
#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous filter-map functions used by FilterMapNode.
///
/// Implementations of this trait define how to filter and map values.
/// The function receives a value and returns `Result<Option<T>, String>` where:
/// - `Ok(Some(value))` - the item is transformed and should be emitted
/// - `Ok(None)` - the item should be filtered out
/// - `Err(error)` - an error occurred during processing
#[async_trait]
pub trait FilterMapFunction: Send + Sync {
  /// Applies the filter-map function to the input value.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value to filter and map
  ///
  /// # Returns
  ///
  /// `Ok(Some(transformed))` if the item should be emitted after transformation,
  /// `Ok(None)` if the item should be filtered out,
  /// `Err(error_message)` if an error occurred during processing.
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Option<Arc<dyn Any + Send + Sync>>, String>;
}

/// Configuration for FilterMapNode that defines the filter-map operation.
///
/// Uses Arc for zero-copy sharing.
pub type FilterMapConfig = Arc<dyn FilterMapFunction>;

/// Wrapper type to send FilterMapConfig through streams.
///
/// Since we can't directly downcast `Arc<dyn Any>` to `Arc<dyn FilterMapFunction>`,
/// we wrap FilterMapConfig in this struct so we can recover it from the stream.
pub struct FilterMapConfigWrapper(pub FilterMapConfig);

impl FilterMapConfigWrapper {
  /// Creates a new wrapper from a FilterMapConfig.
  pub fn new(config: FilterMapConfig) -> Self {
    Self(config)
  }
}

/// Wrapper type that implements FilterMapFunction for async closures.
struct FilterMapFunctionWrapper<F> {
  function: F,
}

#[async_trait]
impl<F, Fut> FilterMapFunction for FilterMapFunctionWrapper<F>
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Option<Arc<dyn Any + Send + Sync>>, String>>
    + Send
    + 'static,
{
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Option<Arc<dyn Any + Send + Sync>>, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a FilterMapConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::stream::{FilterMapConfig, filter_map_config};
///
/// // Create a config that filters even numbers and doubles them
/// let config: FilterMapConfig = filter_map_config(|value| async move {
///     if let Ok(i32_val) = value.downcast::<i32>() {
///         if *i32_val % 2 == 0 {
///             Ok(Some(Arc::new(*i32_val * 2) as Arc<dyn Any + Send + Sync>))
///         } else {
///             Ok(None) // Filter out odd numbers
///         }
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn filter_map_config<F, Fut>(function: F) -> FilterMapConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Option<Arc<dyn Any + Send + Sync>>, String>>
    + Send
    + 'static,
{
  Arc::new(FilterMapFunctionWrapper {
    function: move |value| {
      Box::pin(function(value))
        as std::pin::Pin<
          Box<
            dyn std::future::Future<Output = Result<Option<Arc<dyn Any + Send + Sync>>, String>>
              + Send,
          >,
        >
    },
  })
}

/// Enum to tag messages from different input ports for merging.
#[allow(dead_code)]
enum InputPort {
  In,
  Function,
}

/// A node that applies a filter-map operation to a stream.
///
/// The filter-map function is defined by configuration received on the "function" port.
/// Each item from the "in" port is passed to the function, which can:
/// - Transform and emit the item (return Some)
/// - Filter out the item (return None)
/// - Cause an error (return Err)
pub struct FilterMapNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl FilterMapNode {
  /// Creates a new FilterMapNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `FilterMapNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::FilterMapNode;
  ///
  /// let node = FilterMapNode::new("filter_map".to_string());
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
impl Node for FilterMapNode {
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
      let function_stream = inputs
        .remove("function")
        .ok_or("Missing 'function' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let function_stream = function_stream.map(|item| (InputPort::Function, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(function_stream)
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
        let mut filter_map_function: Option<FilterMapConfig> = None;
        let mut data_buffer: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        // First, collect function and buffer data until we have everything we need
        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Function => {
              // Set filter-map function
              if let Ok(wrapper) = Arc::downcast::<FilterMapConfigWrapper>(item.clone()) {
                filter_map_function = Some(wrapper.0.clone());
                // Now process any buffered data items
                for buffered_item in data_buffer.drain(..) {
                  if let Some(ref func) = filter_map_function {
                    match func.apply(buffered_item).await {
                      Ok(Some(result)) => {
                        let _ = out_tx_clone.send(result).await;
                      }
                      Ok(None) => {
                        // Filtered out - do nothing
                      }
                      Err(e) => {
                        let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                        let _ = error_tx_clone.send(error_arc).await;
                      }
                    }
                  }
                }
              } else {
                let error_msg = format!(
                  "Invalid filter-map function type: {} (expected FilterMapConfigWrapper)",
                  std::any::type_name_of_val(&*item)
                );
                let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                return;
              }
            }
            InputPort::In => {
              if let Some(ref func) = filter_map_function {
                // Process immediately if we have function
                match func.apply(item).await {
                  Ok(Some(result)) => {
                    let _ = out_tx_clone.send(result).await;
                  }
                  Ok(None) => {
                    // Filtered out - do nothing
                  }
                  Err(e) => {
                    let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                    let _ = error_tx_clone.send(error_arc).await;
                  }
                }
              } else {
                // Buffer data items until function is configured
                data_buffer.push(item);
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
