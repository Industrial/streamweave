//! # Common Node Utilities
//!
//! Shared utilities and helpers for node implementations.

use crate::node::InputStream;
use futures::stream;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

/// Message type for distinguishing configuration from data in merged streams.
#[derive(Clone, Copy)]
pub enum MessageType {
  Config,
  Data,
}

/// Type alias for commonly used sender type in tests to reduce type complexity.
pub type TestSender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Type alias for vector of senders used in sync node tests to reduce type complexity.
pub type TestSenderVec = Vec<mpsc::Sender<Arc<dyn Any + Send + Sync>>>;

/// Unified configuration trait for all node types.
///
/// This trait allows nodes to accept configuration in a unified way.
/// Each node type implements this trait for their specific function type.
#[async_trait::async_trait]
pub trait NodeFunction: Send + Sync {
  /// Applies the function to the input value.
  ///
  /// The return type is intentionally generic - each node type defines
  /// what their function returns (transformed value, bool for filter/condition, etc.)
  async fn apply(
    &self,
    value: Arc<dyn Any + Send + Sync>,
  ) -> Result<Arc<dyn Any + Send + Sync>, String>;
}

/// Unified configuration type for all nodes.
///
/// This allows nodes to share the same configuration handling logic
/// without needing separate types for each node.
pub type NodeConfig = Arc<dyn NodeFunction>;

/// Helper function to process a configuration-enabled node with merged config and data streams.
///
/// This function handles the common pattern of:
/// 1. Merging configuration and data streams
/// 2. Processing configuration updates
/// 3. Processing data items with the current configuration
///
/// # Type Parameters
///
/// * `C` - The configuration type (must be `Send + Sync + Clone`)
/// * `F` - A closure that processes data items with the current config
///
/// # Arguments
///
/// * `config_stream` - Stream of configuration updates
/// * `data_stream` - Stream of data items to process
/// * `config_state` - Shared state for storing current configuration
/// * `process_data` - Closure that processes a data item with the current config
///
/// # Returns
///
/// A tuple of (out_rx, error_rx) receivers that can be converted to streams
///
/// # Zero-Copy Guarantee
///
/// This function ensures zero-copy data passing:
/// - Data items are moved into the process_data closure (Arc ownership transfer)
/// - Only Arc references are cloned (atomic refcount increment, ~1-2ns)
/// - No data copying occurs
type OutputReceiver = tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>;

pub fn process_configurable_node<C, F, Fut>(
  config_stream: InputStream,
  data_stream: InputStream,
  config_state: Arc<Mutex<Option<Arc<C>>>>,
  process_data: F,
) -> (OutputReceiver, OutputReceiver)
where
  C: Send + Sync + Clone + 'static,
  F: Fn(Arc<dyn Any + Send + Sync>, &Arc<C>) -> Fut + Send + Sync + Clone + 'static,
  Fut: std::future::Future<Output = Result<Option<Arc<dyn Any + Send + Sync>>, String>> + Send,
{
  // Tag streams to distinguish config from data
  let config_stream = config_stream.map(|item| (MessageType::Config, item));
  let data_stream = data_stream.map(|item| (MessageType::Data, item));

  // Merge streams
  let merged_stream = stream::select(config_stream, data_stream);

  // Create output channels
  let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
  let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

  // Process the merged stream
  let config_state_clone = Arc::clone(&config_state);
  let out_tx_clone = out_tx.clone();
  let error_tx_clone = error_tx.clone();
  let process_data_clone = process_data.clone();

  tokio::spawn(async move {
    let mut merged = merged_stream;
    let mut current_config: Option<Arc<C>> = None;

    while let Some((msg_type, item)) = merged.next().await {
      match msg_type {
        MessageType::Config => {
          // Try to downcast to Arc<C>
          // Since item is Arc<dyn Any>, downcast::<C>() returns Arc<C> if successful
          // When C is Arc<dyn Function>, downcast::<C>() returns Arc<Arc<dyn Function>> = Arc<C>
          // So cfg is already Arc<C>, which is what we need
          if let Ok(cfg) = item.downcast::<C>() {
            // cfg is Arc<C> from downcast
            // When C is Arc<dyn Function>, cfg is Arc<Arc<dyn Function>> = Arc<C>
            current_config = Some(cfg.clone());
            *config_state_clone.lock().await = Some(cfg);
          } else {
            let error_msg: String = format!(
              "Invalid configuration type - expected {}",
              std::any::type_name::<C>()
            );
            let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
            let _ = error_tx_clone.send(error_arc).await;
          }
        }
        MessageType::Data => {
          match &current_config {
            Some(cfg) => {
              // Zero-copy: item is Arc, we move it into the closure (only Arc refcount, no data copy)
              // The process_data closure receives ownership of the Arc
              match process_data_clone(item, cfg).await {
                Ok(Some(output)) => {
                  let _ = out_tx_clone.send(output).await;
                }
                Ok(None) => {
                  // Item was filtered out or handled internally - no output
                }
                Err(error_msg) => {
                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            None => {
              let error_msg: String =
                "No configuration set. Please send configuration before data.".to_string();
              let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
              let _ = error_tx_clone.send(error_arc).await;
            }
          }
        }
      }
    }
  });

  (out_rx, error_rx)
}

/// Base node structure that provides common functionality for all nodes.
///
/// This struct stores the node's name and port names, providing default
/// implementations for the `Node` trait's metadata methods.
pub struct BaseNode {
  pub name: String,
  pub input_port_names: Vec<String>,
  pub output_port_names: Vec<String>,
}

impl BaseNode {
  /// Creates a new BaseNode with the given name and port names.
  pub fn new(name: String, input_port_names: Vec<String>, output_port_names: Vec<String>) -> Self {
    Self {
      name,
      input_port_names,
      output_port_names,
    }
  }

  /// Returns the node's name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Sets the node's name.
  pub fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  /// Returns the list of input port names.
  pub fn input_port_names(&self) -> &[String] {
    &self.input_port_names
  }

  /// Returns the list of output port names.
  pub fn output_port_names(&self) -> &[String] {
    &self.output_port_names
  }

  /// Checks if the node has an input port with the given name.
  pub fn has_input_port(&self, name: &str) -> bool {
    self.input_port_names.contains(&name.to_string())
  }

  /// Checks if the node has an output port with the given name.
  pub fn has_output_port(&self, name: &str) -> bool {
    self.output_port_names.contains(&name.to_string())
  }
}
