//! # Scan Node
#![allow(clippy::type_complexity)]
//!
//! A scan node that applies a configurable reduction function to accumulate values from a stream,
//! emitting intermediate results at each step.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to scan
//! - **Input**: `"initial"` - Receives the initial accumulator value
//! - **Input**: `"function"` - Receives the reduction function configuration
//! - **Output**: `"out"` - Sends intermediate accumulated results at each step
//! - **Output**: `"error"` - Sends errors that occur during scanning
//!
//! ## Behavior
//!
//! The node applies a reduction function to accumulate values from the input stream,
//! emitting each intermediate accumulator value. It starts with an initial value and applies
//! the function: `accumulator = function(accumulator, current_item)` for each item in the stream.
//! Unlike reduce, scan emits the accumulator value after each application of the function.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous reduction functions used by ScanNode.
///
/// Implementations of this trait define how to reduce/accumulate values.
/// The function receives an accumulator and a current value, and returns
/// the new accumulator value.
#[async_trait]
pub trait ScanFunction: Send + Sync {
  /// Applies the reduction function to the accumulator and current value.
  ///
  /// # Arguments
  ///
  /// * `accumulator` - The current accumulator value
  /// * `value` - The current value from the stream
  ///
  /// # Returns
  ///
  /// `Ok(new_accumulator)` if reduction succeeds, or `Err(error_message)` if it fails.
  async fn apply(
    &self,
    accumulator: Arc<dyn Any + Send + Sync>,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String>;
}

/// Configuration for ScanNode that defines the reduction operation.
///
/// Uses Arc for zero-copy sharing.
pub type ScanConfig = Arc<dyn ScanFunction>;

/// Wrapper type to send ScanConfig through streams.
///
/// Since we can't directly downcast `Arc<dyn Any>` to `Arc<dyn ScanFunction>`,
/// we wrap ScanConfig in this struct so we can recover it from the stream.
pub struct ScanConfigWrapper(pub ScanConfig);

impl ScanConfigWrapper {
  /// Creates a new wrapper from a ScanConfig.
  pub fn new(config: ScanConfig) -> Self {
    Self(config)
  }
}

/// Wrapper type that implements ScanFunction for async closures.
struct ScanFunctionWrapper<F> {
  function: F,
}

#[async_trait]
impl<F> ScanFunction for ScanFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
      Arc<dyn Any + Send + Sync>,
    ) -> std::pin::Pin<
      Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>,
    > + Send
    + Sync,
{
  async fn apply(
    &self,
    accumulator: Arc<dyn Any + Send + Sync>,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String> {
    (self.function)(accumulator, value).await
  }
}

/// Helper function to create a ScanConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::nodes::reduction::{ScanConfig, scan_config};
///
/// // Create a config that sums values
/// let config: ScanConfig = scan_config(|acc, value| async move {
///     if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
///         Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn scan_config<F, Fut>(function: F) -> ScanConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send + 'static,
{
  Arc::new(ScanFunctionWrapper {
    function: move |acc, v| {
      Box::pin(function(acc, v))
        as std::pin::Pin<
          Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>,
        >
    },
  })
}

/// Enum to tag input ports
#[allow(dead_code)]
enum InputPort {
  Config,
  In,
  Initial,
  Function,
}

/// A node that applies a configurable reduction function to accumulate values from a stream,
/// emitting intermediate results at each step.
///
/// The reduction function is defined by configuration received on the "function" port.
/// The initial accumulator value is received on the "initial" port.
/// Each item from the "in" port is combined with the accumulator using the reduction function.
/// After each combination, the intermediate accumulator is sent to the "out" port.
pub struct ScanNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
}

impl ScanNode {
  /// Creates a new ScanNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::reduction::ScanNode;
  ///
  /// let node = ScanNode::new("scan".to_string());
  /// // Creates ports: configuration, in, initial, function → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "initial".to_string(),
          "function".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for ScanNode {
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
      let initial_stream = inputs.remove("initial").ok_or("Missing 'initial' input")?;
      let function_stream = inputs
        .remove("function")
        .ok_or("Missing 'function' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let initial_stream = initial_stream.map(|item| (InputPort::Initial, item));
      let function_stream = function_stream.map(|item| (InputPort::Function, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(initial_stream)
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
        let mut accumulator: Option<Arc<dyn Any + Send + Sync>> = None;
        let mut scan_function: Option<ScanConfig> = None;
        let mut data_buffer: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        // First, collect initial value and function, and buffer data items
        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Configuration port is unused for now
            }
            InputPort::Initial => {
              // Set initial value (only use first value)
              if accumulator.is_none() {
                accumulator = Some(item);
              }
            }
            InputPort::Function => {
              // Set scan function
              if scan_function.is_none() {
                if let Ok(wrapper) = Arc::downcast::<ScanConfigWrapper>(item.clone()) {
                  scan_function = Some(wrapper.0.clone());
                } else {
                  let error_msg = format!(
                    "Invalid scan function type: {} (expected ScanConfigWrapper)",
                    std::any::type_name_of_val(&*item)
                  );
                  let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                  return;
                }
              }
            }
            InputPort::In => {
              // Buffer all data items until we have both initial value and function
              data_buffer.push(item);
            }
          }
        }

        // Now process if we have both initial value and function
        if let (Some(acc), Some(func)) = (&accumulator, &scan_function) {
          // Emit initial value first
          let _ = out_tx_clone.send(acc.clone()).await;

          // Then process each buffered data item, emitting intermediate results
          let mut current_acc = acc.clone();
          for item in data_buffer {
            match func.apply(current_acc.clone(), item).await {
              Ok(new_acc) => {
                current_acc = new_acc.clone();
                // Emit intermediate result
                let _ = out_tx_clone.send(new_acc).await;
              }
              Err(e) => {
                let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                let _ = error_tx_clone.send(error_arc).await;
                return;
              }
            }
          }
        } else {
          // Missing initial value or function
          let error_msg = if accumulator.is_none() && scan_function.is_none() {
            "No initial value and no scan function provided".to_string()
          } else if accumulator.is_none() {
            "No initial value provided for scan".to_string()
          } else {
            "No scan function provided".to_string()
          };
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
