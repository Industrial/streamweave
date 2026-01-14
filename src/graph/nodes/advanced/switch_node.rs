//! # Switch Node
//!
//! A transform node that routes data items to different outputs based on a switch value.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define the switch cases
//! - **Input**: `"in"` - Receives data items to route
//! - **Input**: `"value"` - Receives switch value to match against cases
//! - **Output**: `"out_0"`, `"out_1"`, ..., `"out_n"` - Sends data items that match specific cases (dynamic ports)
//! - **Output**: `"default"` - Sends data items that don't match any case
//! - **Output**: `"error"` - Sends errors that occur during switching
//!
//! ## Configuration
//!
//! The configuration port receives `SwitchConfig` (which is `Arc<dyn SwitchFunction>`) that defines
//! the switch cases. The function receives both the data item and the switch value, and returns
//! `Some(branch_index)` to route to `out_{branch_index}`, or `None` to route to the `default` port.

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

/// Trait for asynchronous switch functions used by SwitchNode.
///
/// Implementations of this trait define how to route items based on a switch value.
/// The function receives both the data item and the switch value, and returns
/// `Some(branch_index)` to route to `out_{branch_index}`, or `None` to route to `default`.
#[async_trait]
pub trait SwitchFunction: Send + Sync {
  /// Routes the data item based on the switch value.
  ///
  /// # Arguments
  ///
  /// * `data` - The data item to route
  /// * `value` - The switch value to match against cases
  ///
  /// # Returns
  ///
  /// `Ok(Some(branch_index))` if the switch value matches case at `branch_index` (routes to `out_{branch_index}`),
  /// `Ok(None)` if the switch value doesn't match any case (routes to `default`),
  /// or `Err(error_message)` if an error occurs (routes to `error`).
  async fn apply(
    &self,
    data: Arc<dyn Any + Send + Sync>,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Option<usize>, String>;
}

/// Configuration for SwitchNode that defines the switch function.
///
/// Contains an Arc-wrapped function that implements `SwitchFunction` to perform the routing.
pub type SwitchConfig = Arc<dyn SwitchFunction>;

/// Wrapper type that implements SwitchFunction for async closures.
struct SwitchFunctionWrapper<F> {
  function: F,
}

#[async_trait::async_trait]
impl<F> SwitchFunction for SwitchFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
      Arc<dyn Any + Send + Sync>,
    )
      -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<usize>, String>> + Send>>
    + Send
    + Sync,
{
  async fn apply(
    &self,
    data: Arc<dyn Any + Send + Sync>,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Option<usize>, String> {
    (self.function)(data, value).await
  }
}

/// Helper function to create a SwitchConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::advanced::{SwitchConfig, switch_config};
///
/// // Create a config that routes based on switch value
/// let config: SwitchConfig = switch_config(|data, value| async move {
///     if let Ok(arc_i32) = value.downcast::<i32>() {
///         let n = *arc_i32;
///         if n == 0 {
///             Ok(Some(0)) // Route to out_0
///         } else if n == 1 {
///             Ok(Some(1)) // Route to out_1
///         } else {
///             Ok(None) // Route to default
///         }
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn switch_config<F, Fut>(function: F) -> SwitchConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Option<usize>, String>> + Send + 'static,
{
  Arc::new(SwitchFunctionWrapper { function })
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  Config,
  Data,
  Value,
}

/// A node that routes data items to different outputs based on a switch value.
///
/// The node receives configuration that defines switch cases, and routes
/// each input item to the appropriate output port based on the switch value.
pub struct SwitchNode {
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<SwitchConfig>>>,
  max_branches: usize,
}

impl SwitchNode {
  /// Creates a new SwitchNode with the given name and maximum number of branches.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `max_branches` - Maximum number of switch cases (creates `out_0` through `out_{max_branches-1}`)
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::advanced::SwitchNode;
  ///
  /// let node = SwitchNode::new("switch".to_string(), 3);
  /// // Creates ports: configuration, in, value → out_0, out_1, out_2, default, error
  /// ```
  pub fn new(name: String, max_branches: usize) -> Self {
    let mut output_ports = vec!["default".to_string(), "error".to_string()];
    for i in 0..max_branches {
      output_ports.push(format!("out_{}", i));
    }

    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "value".to_string(),
        ],
        output_ports,
      ),
      current_config: Arc::new(Mutex::new(None)),
      max_branches,
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

  /// Returns the maximum number of branches this node supports.
  pub fn max_branches(&self) -> usize {
    self.max_branches
  }
}

#[async_trait]
impl Node for SwitchNode {
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
    let max_branches = self.max_branches;

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;
      let data_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let value_stream = inputs.remove("value").ok_or("Missing 'value' input")?;

      // Tag streams to distinguish inputs
      let config_stream = config_stream.map(|item| (InputPort::Config, item));
      let data_stream = data_stream.map(|item| (InputPort::Data, item));
      let value_stream = value_stream.map(|item| (InputPort::Value, item));

      // Merge streams
      let merged_stream = stream::select_all(vec![
        Box::pin(config_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(data_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(value_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]);

      // Create output channels for all branches plus default and error
      type ChannelPair = (
        tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
        tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
      );
      let mut output_channels: HashMap<String, ChannelPair> = HashMap::new();

      // Create channels for each branch
      for i in 0..max_branches {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        output_channels.insert(format!("out_{}", i), (tx, rx));
      }

      // Create channels for default and error
      let (default_tx, default_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);
      output_channels.insert("default".to_string(), (default_tx.clone(), default_rx));
      output_channels.insert("error".to_string(), (error_tx.clone(), error_rx));

      // Extract senders for the processing task
      let mut branch_txs: Vec<tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>> = Vec::new();
      for i in 0..max_branches {
        branch_txs.push(
          output_channels
            .get(&format!("out_{}", i))
            .unwrap()
            .0
            .clone(),
        );
      }

      // Process the merged stream
      let mut merged_stream = merged_stream;
      let mut pending_data: Option<Arc<dyn Any + Send + Sync>> = None;
      let mut pending_value: Option<Arc<dyn Any + Send + Sync>> = None;

      tokio::spawn(async move {
        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Update configuration
              if let Ok(arc_config) = item.downcast::<SwitchConfig>() {
                let mut config = config_state.lock().await;
                *config = Some(arc_config);
              }
            }
            InputPort::Data => {
              // Store data item and try to route if we have both data and value
              pending_data = Some(item);
              if let (Some(data), Some(value)) = (pending_data.take(), pending_value.take()) {
                // We have both data and value, try to route
                let config = config_state.lock().await;
                if let Some(switch_fn) = config.as_ref() {
                  match switch_fn.apply(data.clone(), value.clone()).await {
                    Ok(Some(branch_index)) => {
                      if branch_index < max_branches {
                        let _ = branch_txs[branch_index].send(data).await;
                      } else {
                        let error_msg = format!(
                          "Branch index {} out of range (max: {})",
                          branch_index, max_branches
                        );
                        let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
                        let _ = error_tx.send(error_arc).await;
                      }
                    }
                    Ok(None) => {
                      // Route to default
                      let _ = default_tx.send(data).await;
                    }
                    Err(e) => {
                      let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                      let _ = error_tx.send(error_arc).await;
                    }
                  }
                } else {
                  // No config set yet - route to default
                  let _ = default_tx.send(data).await;
                }
              } else {
                // Still waiting for value, restore data
                pending_data = Some(data);
              }
            }
            InputPort::Value => {
              // Store value and try to route if we have both data and value
              pending_value = Some(item);
              if let (Some(data), Some(value)) = (pending_data.take(), pending_value.take()) {
                // We have both data and value, try to route
                let config = config_state.lock().await;
                if let Some(switch_fn) = config.as_ref() {
                  match switch_fn.apply(data.clone(), value.clone()).await {
                    Ok(Some(branch_index)) => {
                      if branch_index < max_branches {
                        let _ = branch_txs[branch_index].send(data).await;
                      } else {
                        let error_msg = format!(
                          "Branch index {} out of range (max: {})",
                          branch_index, max_branches
                        );
                        let error_arc = Arc::new(error_msg) as Arc<dyn Any + Send + Sync>;
                        let _ = error_tx.send(error_arc).await;
                      }
                    }
                    Ok(None) => {
                      // Route to default
                      let _ = default_tx.send(data).await;
                    }
                    Err(e) => {
                      let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                      let _ = error_tx.send(error_arc).await;
                    }
                  }
                } else {
                  // No config set yet - route to default
                  let _ = default_tx.send(data).await;
                }
              } else {
                // Still waiting for data, restore value
                pending_value = Some(value);
              }
            }
          }
        }
      });

      // Convert channels to streams
      let mut outputs = HashMap::new();
      for (port_name, (_tx, rx)) in output_channels {
        outputs.insert(
          port_name.clone(),
          Box::pin(ReceiverStream::new(rx))
            as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
      }

      Ok(outputs)
    })
  }
}
