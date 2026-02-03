//! # Try-Catch Node
//!
//! A transform node that applies a try function to each item and handles errors with a catch function.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to process
//! - **Input**: `"try"` - Receives the try function that processes items (may fail)
//! - **Input**: `"catch"` - Receives the catch function that handles errors
//! - **Output**: `"out"` - Sends successful results or caught error results
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node processes each item from the "in" port by:
//! 1. Applying the try function to the item
//! 2. If successful, sending the result to "out"
//! 3. If it fails, applying the catch function to the error and sending the result to "out"
//! 4. If the catch function also fails, sending the error to "error" port

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;

use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous try functions used by TryCatchNode.
///
/// Implementations of this trait define how to process input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns
/// either `Ok(result)` on success or `Err(error)` on failure.
#[async_trait]
pub trait TryFunction: Any + Send + Sync {
  /// Applies the try function to the input value.
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

/// Trait for asynchronous catch functions used by TryCatchNode.
///
/// Implementations of this trait define how to handle errors.
/// The function receives an error `Arc<dyn Any + Send + Sync>` and returns
/// either `Ok(result)` if the error was handled successfully, or `Err(error)` if handling failed.
#[async_trait]
pub trait CatchFunction: Any + Send + Sync {
  /// Applies the catch function to handle an error.
  ///
  /// # Arguments
  ///
  /// * `error` - The error value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(result)` if error handling succeeds, or `Err(error)` if handling fails.
  async fn apply(
    &self,
    error: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>;
}

/// Configuration for TryCatchNode that defines the try function.
pub type TryConfig = Arc<dyn TryFunction>;

/// Configuration for TryCatchNode that defines the catch function.
pub type CatchConfig = Arc<dyn CatchFunction>;

/// Wrapper type that implements TryFunction for async closures.
struct TryFunctionWrapper<F> {
  /// The async function to wrap.
  function: F,
}

#[async_trait]
impl<F, Fut> TryFunction for TryFunctionWrapper<F>
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
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

/// Wrapper type that implements CatchFunction for async closures.
struct CatchFunctionWrapper<F> {
  /// The async function to wrap.
  function: F,
}

#[async_trait]
impl<F, Fut> CatchFunction for CatchFunctionWrapper<F>
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>>
    + Send,
{
  async fn apply(
    &self,
    error: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>> {
    (self.function)(error).await
  }
}

/// Helper function to create a TryConfig from an async closure.
pub fn try_config<F, Fut>(function: F) -> TryConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>>
    + Send
    + 'static,
{
  Arc::new(TryFunctionWrapper {
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

/// Helper function to create a CatchConfig from an async closure.
pub fn catch_config<F, Fut>(function: F) -> CatchConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>>
    + Send
    + 'static,
{
  Arc::new(CatchFunctionWrapper {
    function: move |e| {
      Box::pin((function)(e))
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

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  /// Input data port.
  In,
  /// Try function configuration port.
  Try,
  /// Catch function configuration port.
  Catch,
}

/// A node that applies a try function to each item and handles errors with a catch function.
///
/// The node processes each item from the "in" port by applying the try function.
/// If the try function succeeds, the result is sent to "out".
/// If it fails, the catch function is applied to the error, and the result is sent to "out".
/// If the catch function also fails, the error is sent to "error" port.
pub struct TryCatchNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Current try function configuration state.
  current_try_config: Arc<Mutex<Option<TryConfig>>>,
  /// Current catch function configuration state.
  current_catch_config: Arc<Mutex<Option<CatchConfig>>>,
}

impl TryCatchNode {
  /// Creates a new TryCatchNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `TryCatchNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::advanced::TryCatchNode;
  ///
  /// let node = TryCatchNode::new("try_catch".to_string());
  /// // Creates ports: configuration, in, try, catch → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "try".to_string(),
          "catch".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_try_config: Arc::new(Mutex::new(None)),
      current_catch_config: Arc::new(Mutex::new(None)),
    }
  }

  /// Returns whether the node has a try configuration set.
  pub fn has_try_config(&self) -> bool {
    self
      .current_try_config
      .try_lock()
      .map(|g| g.is_some())
      .unwrap_or(false)
  }

  /// Returns whether the node has a catch configuration set.
  pub fn has_catch_config(&self) -> bool {
    self
      .current_catch_config
      .try_lock()
      .map(|g| g.is_some())
      .unwrap_or(false)
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for TryCatchNode {
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
    let try_config_state = Arc::clone(&self.current_try_config);
    let catch_config_state = Arc::clone(&self.current_catch_config);

    Box::pin(async move {
      // Extract input streams
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let try_stream = inputs.remove("try").ok_or("Missing 'try' input")?;
      let catch_stream = inputs.remove("catch").ok_or("Missing 'catch' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let try_stream = try_stream.map(|item| (InputPort::Try, item));
      let catch_stream = catch_stream.map(|item| (InputPort::Catch, item));

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process streams with priority for configuration updates
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut try_stream = try_stream;
        let mut catch_stream = catch_stream;
        let mut current_try_config: Option<TryConfig> = None;
        let mut current_catch_config: Option<CatchConfig> = None;

        loop {
          tokio::select! {
            // Prioritize configuration updates
            try_result = try_stream.next() => {
              if let Some((_, item)) = try_result {
                // Update try configuration
                if let Ok(arc_arc_fn) = item.clone().downcast::<Arc<Arc<dyn TryFunction>>>() {
                  current_try_config = Some(Arc::clone(&**arc_arc_fn));
                  let mut config = try_config_state.lock().await;
                  *config = Some(Arc::clone(&**arc_arc_fn));
                } else if let Ok(arc_function) = item.clone().downcast::<Arc<dyn TryFunction>>() {
                  current_try_config = Some(Arc::clone(&*arc_function));
                  let mut config = try_config_state.lock().await;
                  *config = Some(Arc::clone(&*arc_function));
                }
              }
            }
            catch_result = catch_stream.next() => {
              if let Some((_, item)) = catch_result {
                // Update catch configuration
                if let Ok(arc_arc_fn) = item.clone().downcast::<Arc<Arc<dyn CatchFunction>>>() {
                  current_catch_config = Some(Arc::clone(&**arc_arc_fn));
                  let mut config = catch_config_state.lock().await;
                  *config = Some(Arc::clone(&**arc_arc_fn));
                } else if let Ok(arc_function) = item.clone().downcast::<Arc<dyn CatchFunction>>() {
                  current_catch_config = Some(Arc::clone(&*arc_function));
                  let mut config = catch_config_state.lock().await;
                  *config = Some(Arc::clone(&*arc_function));
                }
              }
            }
            // Process data items after configurations
            in_result = in_stream.next() => {
              match in_result {
                Some((_, item)) => {
                  // Process data item
                  if let Some(try_fn) = &current_try_config {
                    match try_fn.apply(item.clone()).await {
                      Ok(result) => {
                        // Try succeeded - send result to out
                        let _ = out_tx_clone.send(result).await;
                      }
                      Err(error) => {
                        // Try failed - apply catch function
                        if let Some(catch_fn) = &current_catch_config {
                          match catch_fn.apply(error).await {
                            Ok(result) => {
                              // Catch succeeded - send result to out
                              let _ = out_tx_clone.send(result).await;
                            }
                            Err(catch_error) => {
                              // Catch also failed - send to error port
                              let _ = error_tx_clone.send(catch_error).await;
                            }
                          }
                        } else {
                          // No catch function - send error to error port
                          let _ = error_tx_clone.send(error).await;
                        }
                      }
                    }
                  } else {
                    // No try function - send item to error port
                    let error_msg =
                      Arc::new("No try function configured".to_string()) as Arc<dyn Any + Send + Sync>;
                    let _ = error_tx_clone.send(error_msg).await;
                  }
                }
                None => {
                  // Input stream ended, check if other streams are also ended
                  if try_stream.next().await.is_none() && catch_stream.next().await.is_none() {
                    break; // All streams ended
                  }
                }
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
