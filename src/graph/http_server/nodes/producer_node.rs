//! # HTTP Server Producer Node
//!
//! Dedicated graph node type for HTTP server producer that converts incoming Axum requests
//! into `Message<HttpServerRequest>` items for processing through a graph.
//!
//! This node implements `NodeTrait` directly (not wrapping a Producer trait) and is designed
//! specifically for graph-based HTTP servers. It receives requests from an internal channel
//! that is set by `GraphServer` via `set_request_receiver()`.

use crate::graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use crate::graph::execution::ExecutionError;
use crate::graph::http_server::nodes::producer::HttpRequestProducerConfig;
use crate::graph::http_server::types::HttpServerRequest;
use crate::graph::traits::{NodeKind, NodeTrait};
use axum::extract::Request;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, warn};

/// Configuration for HTTP server producer node.
#[derive(Debug, Clone, Default)]
pub struct HttpServerProducerConfig {
  /// HTTP request processing configuration
  pub http_config: HttpRequestProducerConfig,
}

/// Dedicated graph node type for HTTP server producer.
///
/// This node:
/// - Receives Axum requests through an internal channel (set by GraphServer)
/// - Converts requests to `Message<HttpServerRequest>`
/// - Sends messages to output channels
/// - Implements `NodeTrait` directly (not a wrapper)
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::graph::http_server::nodes::HttpServerProducerNode;
///
/// let node = HttpServerProducerNode::with_default_config("http_server".to_string());
/// // GraphServer will call set_request_receiver() to connect the channel
/// ```
#[derive(Debug)]
pub struct HttpServerProducerNode {
  /// Node name
  name: String,
  /// Configuration
  config: HttpServerProducerConfig,
  /// Channel receiver for incoming Axum requests (set by GraphServer)
  request_receiver: Arc<Mutex<Option<mpsc::Receiver<Request>>>>,
  /// Output port names (single port "out")
  output_port_names: Vec<String>,
}

impl HttpServerProducerNode {
  /// Creates a new HTTP server producer node with the given name and configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `config` - Configuration for the producer
  ///
  /// # Returns
  ///
  /// A new `HttpServerProducerNode` instance.
  pub fn new(name: String, config: HttpServerProducerConfig) -> Self {
    debug!(node = %name, "HttpServerProducerNode::new()");
    Self {
      name,
      config,
      request_receiver: Arc::new(Mutex::new(None)),
      output_port_names: vec!["out".to_string()],
    }
  }

  /// Creates a new HTTP server producer node with default configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  ///
  /// # Returns
  ///
  /// A new `HttpServerProducerNode` instance with default configuration.
  pub fn with_default_config(name: String) -> Self {
    debug!(node = %name, "HttpServerProducerNode::with_default_config()");
    Self::new(name, HttpServerProducerConfig::default())
  }

  /// Sets the request receiver channel (called by GraphServer).
  ///
  /// # Arguments
  ///
  /// * `receiver` - Channel receiver for incoming Axum requests
  pub(crate) async fn set_request_receiver(&self, receiver: mpsc::Receiver<Request>) {
    debug!(node = %self.name, "HttpServerProducerNode::set_request_receiver()");
    let mut guard = self.request_receiver.lock().await;
    *guard = Some(receiver);
  }

  /// Returns a reference to the configuration.
  #[must_use]
  pub fn config(&self) -> &HttpServerProducerConfig {
    &self.config
  }

  /// Returns a mutable reference to the configuration.
  pub fn config_mut(&mut self) -> &mut HttpServerProducerConfig {
    &mut self.config
  }
}

impl Clone for HttpServerProducerNode {
  fn clone(&self) -> Self {
    Self {
      name: self.name.clone(),
      config: self.config.clone(),
      request_receiver: Arc::clone(&self.request_receiver),
      output_port_names: self.output_port_names.clone(),
    }
  }
}

// DynClone is automatically satisfied by Clone + dyn_clone blanket impls

impl NodeTrait for HttpServerProducerNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Producer
  }

  fn input_port_names(&self) -> Vec<String> {
    vec![] // Producers have no input ports
  }

  fn output_port_names(&self) -> Vec<String> {
    self.output_port_names.clone()
  }

  fn has_input_port(&self, _port_name: &str) -> bool {
    false // Producers have no input ports
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    self.output_port_names.iter().any(|name| name == port_name)
  }

  fn spawn_execution_task(
    &self,
    _input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    _use_shared_memory: bool,
    _arc_pool: Option<std::sync::Arc<crate::graph::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), ExecutionError>>> {
    let node_name = self.name.clone();
    debug!(
      node = %node_name,
      output_channels = output_channels.len(),
      "HttpServerProducerNode::spawn_execution_task()"
    );
    let request_receiver = Arc::clone(&self.request_receiver);

    let handle = tokio::spawn(async move {
      // Get the receiver from the mutex
      let mut receiver_guard = request_receiver.lock().await;
      let mut receiver = match receiver_guard.take() {
        Some(rx) => rx,
        None => {
          error!(
            node = %node_name,
            "Request receiver not set - GraphServer must call set_request_receiver()"
          );
          return Err(ExecutionError::NodeExecutionFailed {
            node: node_name.clone(),
            reason: "Request receiver not set".to_string(),
            message_id: None,
          });
        }
      };

      // Iterate over requests
      loop {
        // Use timeout to periodically check for shutdown
        let request_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await;

        let axum_request = match request_result {
          Ok(Some(req)) => req,
          Ok(None) => {
            // Channel closed, terminate gracefully
            info!(
              node = %node_name,
              "Request channel closed, terminating producer"
            );
            break;
          }
          Err(_) => {
            // Timeout - check if we should exit (shutdown)
            let paused = *pause_signal.read().await;
            if paused {
              // Pause signal set - during shutdown, exit gracefully
              return Ok(());
            }
            // Not paused, continue waiting for request
            continue;
          }
        };

        // Check pause signal before processing each request
        let pause_check_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
            while *pause_signal.read().await {
              tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
          })
          .await;

        // If timeout occurred and we're still paused, might be shutdown
        if pause_check_result.is_err() && *pause_signal.read().await {
          // Still paused after timeout - likely shutdown, exit gracefully
          return Ok(());
        }

        // Convert Axum request to HttpServerRequest
        let http_request = HttpServerRequest::from_axum_request(axum_request).await;

        // Log the incoming request
        tracing::info!(
          node = %node_name,
          request_id = %http_request.request_id,
          method = ?http_request.method,
          path = %http_request.path,
          "Received HTTP request"
        );

        // Wrap in Message<T> before sending through channels
        // Use the request_id from HttpServerRequest as the MessageId for correlation
        let request_id_clone = http_request.request_id.clone();
        let message_id = crate::message::MessageId::new_custom(http_request.request_id.clone());
        let message = crate::message::Message::new(http_request, message_id);
        let message_arc = Arc::new(message);

        // Send to output channels (fan-out if multiple outputs)
        let is_fan_out = output_channels.len() > 1;
        let mut send_succeeded = 0;
        let mut send_failed_count = 0;

        if output_channels.is_empty() {
          warn!(
            node = %node_name,
            request_id = %request_id_clone,
            "No output channels available - message will not be sent"
          );
          continue;
        }

        tracing::info!(
          node = %node_name,
          request_id = %request_id_clone,
          output_channels = output_channels.len(),
          "Sending message to output channels"
        );

        if is_fan_out {
          // Fan-out: clone Arc<Message<T>> (zero-copy) to all outputs
          for (port_name, sender) in &output_channels {
            // Convert Arc<Message<T>> to Arc<dyn Any + Send + Sync> for type erasure
            let arc_any: Arc<dyn Any + Send + Sync> = unsafe {
              Arc::from_raw(Arc::into_raw(message_arc.clone()) as *const (dyn Any + Send + Sync))
            };
            match sender.send(ChannelItem::Arc(arc_any)).await {
              Ok(()) => {
                send_succeeded += 1;
                tracing::info!(
                  node = %node_name,
                  request_id = %request_id_clone,
                  port = %port_name,
                  "Message sent successfully (fan-out)"
                );
              }
              Err(e) => {
                tracing::error!(
                  node = %node_name,
                  request_id = %request_id_clone,
                  port = %port_name,
                  error = %e,
                  "Failed to send message"
                );
                let paused = *pause_signal.read().await;
                if paused {
                  return Ok(());
                }
                send_failed_count += 1;
                warn!(
                  node = %node_name,
                  port = %port_name,
                  "Output channel receiver dropped (may be normal in fan-out scenarios)"
                );
              }
            }
          }
        } else {
          // Single output: send Arc<Message<T>> directly
          let (port_name, sender) = output_channels.iter().next().unwrap();
          // Convert Arc<Message<T>> to Arc<dyn Any + Send + Sync> for type erasure
          let arc_any: Arc<dyn Any + Send + Sync> =
            unsafe { Arc::from_raw(Arc::into_raw(message_arc) as *const (dyn Any + Send + Sync)) };
          match sender.send(ChannelItem::Arc(arc_any)).await {
            Ok(()) => {
              send_succeeded += 1;
              tracing::info!(
                node = %node_name,
                request_id = %request_id_clone,
                port = %port_name,
                "Message sent successfully (single output)"
              );
            }
            Err(e) => {
              tracing::error!(
                node = %node_name,
                request_id = %request_id_clone,
                port = %port_name,
                error = %e,
                "Failed to send message"
              );
              let paused = *pause_signal.read().await;
              if paused {
                return Ok(());
              }
              send_failed_count += 1;
              warn!(
                node = %node_name,
                port = port_name,
                "Output channel receiver dropped"
              );
            }
          }
        }

        // If ALL channels failed (and not shutdown), all downstream nodes have finished
        if send_failed_count > 0 && send_succeeded == 0 {
          // All downstream nodes finished - exit gracefully
          return Ok(());
        }
      }

      Ok(())
    });

    Some(handle)
  }
}
