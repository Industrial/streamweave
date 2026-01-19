//! # Aggregate Node
#![allow(clippy::type_complexity)]
//!
//! A general-purpose aggregation node that applies a configurable aggregator function to process items from a stream.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to aggregate
//! - **Input**: `"aggregator"` - Receives the aggregator function configuration
//! - **Output**: `"out"` - Sends the aggregated result
//! - **Output**: `"error"` - Sends errors that occur during aggregation
//!
//! ## Behavior
//!
//! The node applies an aggregator function to process items from the input stream.
//! The aggregator function processes items incrementally and maintains its own state.
//! When the stream ends, the aggregator's finalize method is called to get the result.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous aggregator functions used by AggregateNode.
///
/// Implementations of this trait define how to aggregate values from a stream.
/// The aggregator processes items incrementally and maintains its own state.
#[async_trait]
pub trait AggregatorFunction: Send + Sync {
  /// Processes an item and updates the aggregator's internal state.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value to process
  ///
  /// # Returns
  ///
  /// `Ok(())` if processing succeeds, or `Err(error_message)` if it fails.
  async fn process_item(&mut self, value: Arc<dyn Any + Send + Sync>) -> Result<(), String>;

  /// Finalizes the aggregation and returns the result.
  ///
  /// This is called when the input stream ends.
  ///
  /// # Returns
  ///
  /// `Ok(result)` if finalization succeeds, or `Err(error_message)` if it fails.
  async fn finalize(&self) -> Result<Arc<dyn Any + Send + Sync>, String>;
}

/// Configuration for AggregateNode that defines the aggregation operation.
///
/// Uses Arc for zero-copy sharing. The aggregator is wrapped in Arc<Mutex<>> internally
/// to allow mutable access during processing.
pub type AggregateConfig = Arc<tokio::sync::Mutex<Box<dyn AggregatorFunction>>>;

/// Wrapper type to send AggregateConfig through streams.
///
/// Since we can't directly downcast `Arc<dyn Any>` to `AggregateConfig`,
/// we wrap it in this struct so we can recover it from the stream.
pub struct AggregateConfigWrapper(pub AggregateConfig);

impl AggregateConfigWrapper {
  /// Creates a new wrapper from an AggregateConfig.
  pub fn new(config: AggregateConfig) -> Self {
    Self(config)
  }
}

/// Helper function to create an AggregateConfig from an aggregator implementation.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::reduction::{AggregateConfig, aggregate_config};
///
/// // Create a config with a custom aggregator
/// struct MyAggregator {
///   sum: i32,
/// }
///
/// #[async_trait::async_trait]
/// impl AggregatorFunction for MyAggregator {
///   async fn process_item(&mut self, value: Arc<dyn Any + Send + Sync>) -> Result<(), String> {
///     if let Ok(arc_i32) = value.downcast::<i32>() {
///       self.sum += *arc_i32;
///       Ok(())
///     } else {
///       Err("Expected i32".to_string())
///     }
///   }
///
///   async fn finalize(&self) -> Result<Arc<dyn Any + Send + Sync>, String> {
///     Ok(Arc::new(self.sum) as Arc<dyn Any + Send + Sync>)
///   }
/// }
///
/// let config: AggregateConfig = aggregate_config(MyAggregator { sum: 0 });
/// ```
pub fn aggregate_config<A>(aggregator: A) -> AggregateConfig
where
  A: AggregatorFunction + 'static,
{
  Arc::new(tokio::sync::Mutex::new(Box::new(aggregator)))
}

/// Enum to tag input ports
#[allow(dead_code)]
enum InputPort {
  Config,
  In,
  Aggregator,
}

/// A node that applies a configurable aggregator function to process items from a stream.
///
/// The aggregator function is defined by configuration received on the "aggregator" port.
/// Each item from the "in" port is processed by the aggregator, which maintains its own state.
/// When the stream ends, the aggregator's finalize method is called to get the result.
pub struct AggregateNode {
  pub(crate) base: BaseNode,
}

impl AggregateNode {
  /// Creates a new AggregateNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::reduction::AggregateNode;
  ///
  /// let node = AggregateNode::new("aggregate".to_string());
  /// // Creates ports: configuration, in, aggregator → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "aggregator".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for AggregateNode {
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
      let aggregator_stream = inputs
        .remove("aggregator")
        .ok_or("Missing 'aggregator' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let aggregator_stream = aggregator_stream.map(|item| (InputPort::Aggregator, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(aggregator_stream)
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
        let mut aggregator: Option<AggregateConfig> = None;

        // First, collect aggregator and buffer data items
        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Configuration port is unused for now
            }
            InputPort::Aggregator => {
              // Set aggregator
              if let Ok(wrapper) = Arc::downcast::<AggregateConfigWrapper>(item.clone()) {
                aggregator = Some(wrapper.0.clone());
              } else {
                let error_msg = format!(
                  "Invalid aggregator type: {} (expected AggregateConfigWrapper)",
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

        // Now process buffered data items if we have aggregator
        if let Some(agg) = aggregator {
          // Process each item
          for item in data_buffer {
            let mut agg_guard = agg.lock().await;
            match agg_guard.process_item(item).await {
              Ok(()) => {
                // Item processed successfully
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                // Continue processing other items even if one fails
              }
            }
          }

          // Finalize and emit result
          let agg_guard = agg.lock().await;
          match agg_guard.finalize().await {
            Ok(result) => {
              let _ = out_tx_clone.send(result).await;
            }
            Err(e) => {
              let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
              let _ = error_tx_clone.send(error_arc).await;
            }
          }
        } else {
          // No aggregator provided
          let error_msg = "No aggregator provided for aggregation".to_string();
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
