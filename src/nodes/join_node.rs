//! # Join Node
//!
//! A node that joins two streams on keys, similar to SQL joins.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that specify join strategy and key extraction
//! - **Input**: `"left"` - Receives items from the left stream
//! - **Input**: `"right"` - Receives items from the right stream
//! - **Output**: `"out"` - Sends joined results
//! - **Output**: `"error"` - Sends errors that occur during joining
//!
//! ## Behavior
//!
//! The node joins two streams based on keys extracted from each item. It supports multiple join strategies:
//! - **Inner Join**: Only emits results when keys match in both streams
//! - **Left Join**: Emits all items from the left stream, with matched items from the right stream (or null)
//! - **Right Join**: Emits all items from the right stream, with matched items from the left stream (or null)
//! - **Outer Join**: Emits all items from both streams, with matches where available
//!
//! The node buffers items from one stream and matches them against items from the other stream.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Join strategy for JoinNode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinStrategy {
  /// Inner join: only emit when keys match in both streams
  Inner,
  /// Left join: emit all left items, with matched right items (or null)
  Left,
  /// Right join: emit all right items, with matched left items (or null)
  Right,
  /// Outer join: emit all items from both streams
  Outer,
}

/// Trait for asynchronous key extraction functions used by JoinNode.
#[async_trait]
pub trait JoinKeyFunction: Send + Sync {
  /// Extracts a key from an item. Returns the key as a String.
  async fn extract_key(&self, value: Arc<dyn Any + Send + Sync>) -> Result<String, String>;
}

/// Trait for asynchronous join result combination functions.
#[async_trait]
pub trait JoinCombineFunction: Send + Sync {
  /// Combines a left and right item into a joined result.
  /// If `right` is None, it means no match was found (for left/outer joins).
  async fn combine(
    &self,
    left: Arc<dyn Any + Send + Sync>,
    right: Option<Arc<dyn Any + Send + Sync>>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String>;
}

/// Configuration for JoinNode.
pub struct JoinConfig {
  /// Join strategy to use
  pub strategy: JoinStrategy,
  /// Function to extract keys from left stream items
  pub left_key_fn: Arc<dyn JoinKeyFunction>,
  /// Function to extract keys from right stream items
  pub right_key_fn: Arc<dyn JoinKeyFunction>,
  /// Function to combine matched items
  pub combine_fn: Arc<dyn JoinCombineFunction>,
}

/// Wrapper type that implements JoinKeyFunction for async closures.
struct JoinKeyFunctionWrapper<F> {
  function: F,
}

#[async_trait::async_trait]
impl<F> JoinKeyFunction for JoinKeyFunctionWrapper<F>
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

/// Wrapper type that implements JoinCombineFunction for async closures.
struct JoinCombineFunctionWrapper<F> {
  function: F,
}

#[async_trait::async_trait]
impl<F> JoinCombineFunction for JoinCombineFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
      Option<Arc<dyn Any + Send + Sync>>,
    ) -> std::pin::Pin<
      Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>,
    > + Send
    + Sync,
{
  async fn combine(
    &self,
    left: Arc<dyn Any + Send + Sync>,
    right: Option<Arc<dyn Any + Send + Sync>>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String> {
    (self.function)(left, right).await
  }
}

/// Helper function to create a JoinConfig from async closures.
pub fn join_config<FKL, FKR, FC, FutK, FutC>(
  strategy: JoinStrategy,
  left_key_fn: FKL,
  right_key_fn: FKR,
  combine_fn: FC,
) -> Arc<JoinConfig>
where
  FKL: Fn(Arc<dyn Any + Send + Sync>) -> FutK + Send + Sync + 'static,
  FKR: Fn(Arc<dyn Any + Send + Sync>) -> FutK + Send + Sync + 'static,
  FC: Fn(Arc<dyn Any + Send + Sync>, Option<Arc<dyn Any + Send + Sync>>) -> FutC
    + Send
    + Sync
    + 'static,
  FutK: std::future::Future<Output = Result<String, String>> + Send + 'static,
  FutC: std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send + 'static,
{
  Arc::new(JoinConfig {
    strategy,
    left_key_fn: Arc::new(JoinKeyFunctionWrapper {
      function: move |v| {
        Box::pin(left_key_fn(v))
          as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
      },
    }),
    right_key_fn: Arc::new(JoinKeyFunctionWrapper {
      function: move |v| {
        Box::pin(right_key_fn(v))
          as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
      },
    }),
    combine_fn: Arc::new(JoinCombineFunctionWrapper {
      function: move |l, r| {
        Box::pin(combine_fn(l, r))
          as std::pin::Pin<
            Box<
              dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send,
            >,
          >
      },
    }),
  })
}

/// A node that joins two streams on keys.
///
/// This node provides SQL-like join functionality, supporting inner, left, right, and outer joins.
pub struct JoinNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<JoinConfig>>>>,
}

impl JoinNode {
  /// Creates a new JoinNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::JoinNode;
  ///
  /// let node = JoinNode::new("join".to_string());
  /// // Creates ports: configuration, left, right → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "left".to_string(),
          "right".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
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
impl Node for JoinNode {
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
      let left_stream = inputs.remove("left").ok_or("Missing 'left' input")?;
      let right_stream = inputs.remove("right").ok_or("Missing 'right' input")?;

      // Tag streams to distinguish message types
      let config_stream = config_stream.map(|item| (MessageType::Config, item));
      let left_stream = left_stream.map(|item| (MessageType::Data, "left".to_string(), item));
      let right_stream = right_stream.map(|item| (MessageType::Data, "right".to_string(), item));

      // Merge all streams
      let merged_stream = stream::select_all(vec![
        Box::pin(config_stream.map(|(t, i)| (t, "config".to_string(), i)))
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(left_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(right_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
      ]);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut current_config: Option<Arc<JoinConfig>> = None;

        // Buffers for left and right streams
        // Key -> Vec of items (to handle multiple items with same key)
        let mut left_buffer: HashMap<String, Vec<Arc<dyn Any + Send + Sync>>> = HashMap::new();
        let mut right_buffer: HashMap<String, Vec<Arc<dyn Any + Send + Sync>>> = HashMap::new();

        // Track which items have been matched (for outer join)
        let mut left_matched: HashMap<String, bool> = HashMap::new();
        let mut right_matched: HashMap<String, bool> = HashMap::new();

        while let Some((msg_type, port_name, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration
              if let Ok(arc_config) = item.clone().downcast::<Arc<JoinConfig>>() {
                current_config = Some(Arc::clone(&*arc_config));
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_config));
              } else {
                let error_msg = "Invalid configuration type - expected Arc<JoinConfig>".to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              if let Some(config) = &current_config {
                match port_name.as_str() {
                  "left" => {
                    // Extract key from left item
                    match config.left_key_fn.extract_key(item.clone()).await {
                      Ok(key) => {
                        // Add to left buffer
                        left_buffer
                          .entry(key.clone())
                          .or_default()
                          .push(item.clone());

                        // Try to match with right buffer
                        if let Some(right_items) = right_buffer.get(&key) {
                          for right_item in right_items {
                            match config
                              .combine_fn
                              .combine(item.clone(), Some(right_item.clone()))
                              .await
                            {
                              Ok(joined) => {
                                let _ = out_tx_clone.send(joined).await;
                              }
                              Err(e) => {
                                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(e);
                                let _ = error_tx_clone.send(error_arc).await;
                              }
                            }
                          }
                          left_matched.insert(key.clone(), true);
                          right_matched.insert(key.clone(), true);
                        } else {
                          // No match in right buffer
                          match config.strategy {
                            JoinStrategy::Left | JoinStrategy::Outer => {
                              // Emit with None for right
                              match config.combine_fn.combine(item.clone(), None).await {
                                Ok(joined) => {
                                  let _ = out_tx_clone.send(joined).await;
                                }
                                Err(e) => {
                                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(e);
                                  let _ = error_tx_clone.send(error_arc).await;
                                }
                              }
                            }
                            _ => {}
                          }
                        }
                      }
                      Err(e) => {
                        let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(e);
                        let _ = error_tx_clone.send(error_arc).await;
                      }
                    }
                  }
                  "right" => {
                    // Extract key from right item
                    match config.right_key_fn.extract_key(item.clone()).await {
                      Ok(key) => {
                        // Add to right buffer
                        right_buffer
                          .entry(key.clone())
                          .or_default()
                          .push(item.clone());

                        // Try to match with left buffer
                        if let Some(left_items) = left_buffer.get(&key) {
                          for left_item in left_items {
                            match config
                              .combine_fn
                              .combine(left_item.clone(), Some(item.clone()))
                              .await
                            {
                              Ok(joined) => {
                                let _ = out_tx_clone.send(joined).await;
                              }
                              Err(e) => {
                                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(e);
                                let _ = error_tx_clone.send(error_arc).await;
                              }
                            }
                          }
                          left_matched.insert(key.clone(), true);
                          right_matched.insert(key.clone(), true);
                        } else {
                          // No match in left buffer
                          match config.strategy {
                            JoinStrategy::Right | JoinStrategy::Outer => {
                              // Emit with None for left
                              match config.combine_fn.combine(item.clone(), None).await {
                                Ok(joined) => {
                                  let _ = out_tx_clone.send(joined).await;
                                }
                                Err(e) => {
                                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(e);
                                  let _ = error_tx_clone.send(error_arc).await;
                                }
                              }
                            }
                            _ => {}
                          }
                        }
                      }
                      Err(e) => {
                        let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(e);
                        let _ = error_tx_clone.send(error_arc).await;
                      }
                    }
                  }
                  _ => {}
                }
              } else {
                // No configuration set
                let error_msg =
                  "No configuration set. Please send configuration before data.".to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_clone.send(error_arc).await;
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
