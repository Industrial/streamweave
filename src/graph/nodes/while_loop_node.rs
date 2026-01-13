//! # WhileLoop Node
//!
//! A transform node that repeats processing of data items until a condition evaluates to false.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define the condition predicate and max iterations
//! - **Input**: `"in"` - Receives data items to process in the loop
//! - **Input**: `"condition"` - Receives break signals to exit the loop early
//! - **Output**: `"out"` - Sends processed data items after the loop completes
//! - **Output**: `"break"` - Sends items when loop exits via break signal
//! - **Output**: `"error"` - Sends errors that occur during loop processing
//!
//! ## Configuration
//!
//! The configuration port receives `WhileLoopConfig` that defines:
//! - The condition predicate function that determines when to continue looping
//! - Maximum number of iterations to prevent infinite loops (default: 1000)
//!
//! The node processes each input item by repeatedly evaluating the condition. If the condition
//! is true, it continues looping. If false, it exits and outputs the item. If a break signal
//! is received on the "condition" port, it exits immediately.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Configuration for WhileLoopNode that defines the loop behavior.
#[derive(Clone)]
pub struct WhileLoopConfig {
  /// The condition function that determines when to continue looping.
  /// Returns `Ok(true)` to continue, `Ok(false)` to exit, or `Err` on error.
  pub condition: Arc<dyn WhileLoopConditionFunction>,
  /// Maximum number of iterations to prevent infinite loops.
  /// Default: 1000
  pub max_iterations: usize,
}

/// Trait for asynchronous condition functions used by WhileLoopNode.
///
/// The function receives the current data item and returns:
/// - `Ok(true)` to continue the loop
/// - `Ok(false)` to exit the loop
/// - `Err(error_message)` if an error occurs
#[async_trait]
pub trait WhileLoopConditionFunction: Send + Sync {
  /// Evaluates the condition to determine if the loop should continue.
  ///
  /// # Arguments
  ///
  /// * `value` - The current data item wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(true)` to continue looping, `Ok(false)` to exit, or `Err(error_message)` on error.
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<bool, String>;
}

/// Wrapper type that implements WhileLoopConditionFunction for async closures.
struct WhileLoopConditionFunctionWrapper<F> {
  function: F,
}

#[async_trait]
impl<F> WhileLoopConditionFunction for WhileLoopConditionFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send>>
    + Send
    + Sync,
{
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<bool, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a WhileLoopConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::{WhileLoopConfig, while_loop_config};
///
/// // Create a config that loops while value is less than 10
/// let config: WhileLoopConfig = while_loop_config(
///     |value| async move {
///         if let Ok(arc_i32) = value.downcast::<i32>() {
///             Ok(*arc_i32 < 10)
///         } else {
///             Err("Expected i32".to_string())
///         }
///     },
///     1000, // max iterations
/// );
/// ```
pub fn while_loop_config<F, Fut>(function: F, max_iterations: usize) -> WhileLoopConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<bool, String>> + Send + 'static,
{
  WhileLoopConfig {
    condition: Arc::new(WhileLoopConditionFunctionWrapper {
      function: move |v| {
        Box::pin(function(v))
          as std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send>>
      },
    }),
    max_iterations,
  }
}

/// A node that repeats processing of data items until a condition is false.
///
/// The node processes each input item by repeatedly evaluating a condition function.
/// If the condition is true, it continues looping. If false, it exits and outputs the item.
/// Supports a break signal to exit early and has a maximum iteration limit to prevent infinite loops.
pub struct WhileLoopNode {
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<WhileLoopConfig>>>>,
}

impl WhileLoopNode {
  /// Creates a new WhileLoopNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::WhileLoopNode;
  ///
  /// let node = WhileLoopNode::new("while_loop".to_string());
  /// // Creates ports: configuration, in, condition → out, break, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "condition".to_string(),
        ],
        vec!["out".to_string(), "break".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
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
impl Node for WhileLoopNode {
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

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;
      let data_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let break_stream = inputs
        .remove("condition")
        .ok_or("Missing 'condition' input")?;

      // Use a broadcast channel for break signals so multiple tasks can listen
      let (break_signal_tx, _) = tokio::sync::broadcast::channel(10);

      // Spawn a task to forward break signals to the broadcast channel
      let break_signal_tx_clone = break_signal_tx.clone();
      tokio::spawn(async move {
        let mut break_stream = break_stream;
        while let Some(_item) = break_stream.next().await {
          let _ = break_signal_tx_clone.send(());
        }
      });

      // Tag streams to distinguish config and data
      let config_stream = config_stream.map(|item| (MessageType::Config, item));
      let data_stream = data_stream.map(|item| (MessageType::Data, item));

      // Merge config and data streams
      let merged_stream = stream::select(config_stream, data_stream);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (break_tx, break_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let out_tx_clone = out_tx.clone();
      let break_tx_clone = break_tx.clone();
      let error_tx_clone = error_tx.clone();
      let break_signal_tx_clone = break_signal_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut current_config: Option<Arc<WhileLoopConfig>> = None;
        let mut current_item: Option<Arc<dyn Any + Send + Sync>> = None;

        // Main processing loop
        loop {
          tokio::select! {
            // Handle messages from merged stream
            msg_opt = merged.next() => {
              if let Some((msg_type, item)) = msg_opt {
                match msg_type {
                  MessageType::Config => {
                    // Update configuration
                    if let Ok(arc_config) = item.clone().downcast::<Arc<WhileLoopConfig>>() {
                      current_config = Some(Arc::clone(&*arc_config));
                      *config_state_clone.lock().await = Some(Arc::clone(&*arc_config));
                    } else {
                      // Invalid configuration type
                      let error_msg: String = "Invalid configuration type - expected Arc<WhileLoopConfig>".to_string();
                      let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                      let _ = error_tx_clone.send(error_arc).await;
                    }
                  }
                  MessageType::Data => {
                    // Start processing a new item - only if we're not already processing one
                    if current_item.is_none() {
                      current_item = Some(item);

                      // Spawn a task to process this item in a while loop
                      let item_clone = current_item.clone().unwrap();
                      let config_clone = current_config.clone();
                      let out_tx_task = out_tx_clone.clone();
                      let break_tx_task = break_tx_clone.clone();
                      let error_tx_task = error_tx_clone.clone();
                      let mut break_signal_rx_task = break_signal_tx_clone.subscribe();

                      tokio::spawn(async move {
                        if let Some(config) = config_clone {
                          // Process the item in a while loop
                          let item = item_clone;
                          let mut iter_count = 0;

                          loop {
                            // Check for break signal (non-blocking)
                            if break_signal_rx_task.try_recv().is_ok() {
                              // Exit loop via break
                              let _ = break_tx_task.send(item).await;
                              break;
                            }

                            // Check max iterations
                            if iter_count >= config.max_iterations {
                              let error_msg: String = format!(
                                "Maximum iterations ({}) exceeded. Possible infinite loop.",
                                config.max_iterations
                              );
                              let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                              let _ = error_tx_task.send(error_arc).await;
                              break;
                            }

                            // Evaluate condition
                            match config.condition.apply(item.clone()).await {
                              Ok(true) => {
                                // Condition is true - continue looping
                                iter_count += 1;
                              }
                              Ok(false) => {
                                // Condition is false - exit loop and output item
                                let _ = out_tx_task.send(item).await;
                                break;
                              }
                              Err(error_msg) => {
                                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                                let _ = error_tx_task.send(error_arc).await;
                                break;
                              }
                            }
                          }
                        } else {
                          // No config - send error
                          let error_msg: String = "No configuration set. Please send configuration before data.".to_string();
                          let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                          let _ = error_tx_task.send(error_arc).await;
                        }
                      });

                      // Clear current_item so we can process the next one
                      current_item = None;
                    }
                  }
                }
              } else {
                // Stream ended
                break;
              }
            }
            // Break signals are handled in spawned tasks via broadcast channel
            // No need to handle them here
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
        "break".to_string(),
        Box::pin(ReceiverStream::new(break_rx))
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
