//! # Retry Node
//!
//! A transform node that retries failed operations with exponential backoff.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to process
//! - **Input**: `"max_retries"` - Receives the maximum number of retry attempts (numeric value)
//! - **Output**: `"out"` - Sends successful results after retries
//! - **Output**: `"error"` - Sends errors that occur after all retries are exhausted
//!
//! ## Behavior
//!
//! The node processes each item from the "in" port by:
//! 1. Applying the retry function (from configuration) to the item
//! 2. If successful, sending the result to "out"
//! 3. If it fails, retrying up to max_retries times with exponential backoff
//! 4. After all retries exhausted, sending the error to "error" port
//!
//! Exponential backoff: delay = base_delay * (2 ^ retry_count)
//! Base delay is 100ms by default.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous retry functions used by RetryNode.
///
/// Implementations of this trait define how to process input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns
/// either `Ok(result)` on success or `Err(error)` on failure.
#[async_trait]
pub trait RetryFunction: Send + Sync {
  /// Applies the retry function to the input value.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(result)` if processing succeeds, or `Err(error)` if it fails.
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>;
}

/// Configuration for RetryNode that defines the retry function.
pub type RetryConfig = Arc<dyn RetryFunction>;

/// Wrapper type that implements RetryFunction for async closures.
struct RetryFunctionWrapper<F> {
  function: F,
}

#[async_trait]

impl<F, Fut> RetryFunction for RetryFunctionWrapper<F>
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>>
    + Send,
{
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>> {
    (self.function)(value).await
  }
}

/// Helper function to create a RetryConfig from an async closure.
pub fn retry_config<F, Fut>(function: F) -> RetryConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>>
    + Send
    + 'static,
{
  Arc::new(RetryFunctionWrapper {
    function: move |v| {
      Box::pin((function)(v))
        as Pin<
          Box<
            dyn std::future::Future<
                Output = Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>,
              > + Send,
          >,
        >
    },
  })
}

/// Helper function to extract usize from various numeric types.
fn get_usize(value: &Arc<dyn Any + Send + Sync>) -> Result<usize, String> {
  if let Ok(arc_usize) = value.clone().downcast::<usize>() {
    Ok(*arc_usize)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Max retries cannot be negative".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Max retries too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Max retries cannot be negative".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Max retries too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    (*arc_u32)
      .try_into()
      .map_err(|_| "Max retries too large".to_string())
  } else if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    (*arc_u64)
      .try_into()
      .map_err(|_| "Max retries too large".to_string())
  } else {
    Err(format!(
      "Unsupported type for max_retries: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum InputPort {
  Config,
  In,
  MaxRetries,
}

/// A node that retries failed operations with exponential backoff.
///
/// The node processes each item from the "in" port by applying the retry function.
/// If the function fails, it retries up to max_retries times with exponential backoff.
/// Base delay is 100ms, and each retry doubles the delay: 100ms, 200ms, 400ms, etc.
pub struct RetryNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current configuration state.
  current_config: Arc<Mutex<Option<RetryConfig>>>,
  base_delay_ms: u64,
}

impl RetryNode {
  /// Creates a new RetryNode with the given name and base delay.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  /// * `base_delay_ms` - Base delay in milliseconds for exponential backoff (default: 100ms).
  ///
  /// # Returns
  ///
  /// A new `RetryNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::advanced::RetryNode;
  ///
  /// let node = RetryNode::new("retry".to_string(), 100);
  /// // Creates ports: configuration, in, max_retries → out, error
  /// ```
  pub fn new(name: String, base_delay_ms: u64) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "max_retries".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
      base_delay_ms,
    }
  }

  /// Returns whether the node has a configuration set.
  pub fn has_config(&self) -> bool {
    self
      .current_config
      .try_lock()
      .map(|g| g.is_some())
      .unwrap_or(false)
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for RetryNode {
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
    let config_state = Arc::clone(&self.current_config);
    let base_delay_ms = self.base_delay_ms;

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let max_retries_stream = inputs
        .remove("max_retries")
        .ok_or("Missing 'max_retries' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let max_retries_stream = max_retries_stream.map(|item| (InputPort::MaxRetries, item));

      // Merge streams - include config stream if present
      let mut streams: Vec<
        Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      > = vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(max_retries_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ];

      if let Some(config_stream) = config_stream {
        let config_stream = config_stream.map(|item| (InputPort::Config, item));
        streams.push(Box::pin(config_stream)
          as Pin<
            Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
          >);
      }

      let merged_stream = stream::select_all(streams);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged_stream = merged_stream;
        let mut current_config: Option<RetryConfig> = None;
        let mut max_retries: Option<usize> = None;
        let mut pending_items: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Update configuration
              if let Ok(arc_arc_fn) = item.clone().downcast::<Arc<Arc<dyn RetryFunction>>>() {
                current_config = Some(Arc::clone(&**arc_arc_fn));
                let mut config = config_state.lock().await;
                *config = Some(Arc::clone(&**arc_arc_fn));
              } else if let Ok(arc_function) = item.clone().downcast::<Arc<dyn RetryFunction>>() {
                current_config = Some(Arc::clone(&*arc_function));
                let mut config = config_state.lock().await;
                *config = Some(Arc::clone(&*arc_function));
              }
            }
            InputPort::MaxRetries => {
              // Update max_retries
              match get_usize(&item) {
                Ok(count) => {
                  max_retries = Some(count);
                  // Process any pending items now that we have max_retries
                  if let Some(current_max_retries) = max_retries
                    && let Some(ref retry_fn) = current_config
                  {
                    for pending_item in pending_items.drain(..) {
                      process_item_with_retry(
                        &pending_item,
                        &Some(retry_fn.clone()),
                        current_max_retries,
                        base_delay_ms,
                        &out_tx_clone,
                        &error_tx_clone,
                      )
                      .await;
                    }
                  }
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Process data item
              if let Some(current_max_retries) = max_retries {
                if let Some(ref retry_fn) = current_config {
                  process_item_with_retry(
                    &item,
                    &Some(retry_fn.clone()),
                    current_max_retries,
                    base_delay_ms,
                    &out_tx_clone,
                    &error_tx_clone,
                  )
                  .await;
                } else {
                  // No config set yet - buffer item
                  pending_items.push(item);
                }
              } else {
                // No max_retries set yet - buffer item
                pending_items.push(item);
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

/// Helper function to process an item with retry logic and exponential backoff.
async fn process_item_with_retry(
  item: &Arc<dyn Any + Send + Sync>,
  config: &Option<RetryConfig>,
  max_retries: usize,
  base_delay_ms: u64,
  out_tx: &tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  error_tx: &tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
) {
  if let Some(retry_fn) = config {
    #[allow(unused_assignments)]
    let mut last_error: Option<Arc<dyn Any + Send + Sync>> = None;

    // Try initial attempt
    match retry_fn.apply(item.clone()).await {
      Ok(result) => {
        let _ = out_tx.send(result).await;
        return;
      }
      Err(error) => {
        last_error = Some(error);
      }
    }

    // Retry with exponential backoff
    for retry_count in 0..max_retries {
      // Calculate exponential backoff delay: base_delay * (2 ^ retry_count)
      let delay_ms = base_delay_ms * (1 << retry_count);
      let delay = Duration::from_millis(delay_ms);

      // Wait before retrying
      sleep(delay).await;

      // Retry the operation
      match retry_fn.apply(item.clone()).await {
        Ok(result) => {
          let _ = out_tx.send(result).await;
          return;
        }
        Err(error) => {
          last_error = Some(error);
        }
      }
    }

    // All retries exhausted - send error
    if let Some(error) = last_error {
      let _ = error_tx.send(error).await;
    } else {
      let error_msg = Arc::new("All retries exhausted".to_string()) as Arc<dyn Any + Send + Sync>;
      let _ = error_tx.send(error_msg).await;
    }
  } else {
    // No config set - send error
    let error_msg =
      Arc::new("No retry function configured".to_string()) as Arc<dyn Any + Send + Sync>;
    let _ = error_tx.send(error_msg).await;
  }
}
