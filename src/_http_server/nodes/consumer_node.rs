//! # HTTP Server Consumer Node
//!
//! Dedicated graph node type for HTTP server consumer that receives `Message<HttpServerResponse>`
//! items from the graph and sends them back to Axum clients.
//!
//! This node implements `NodeTrait` directly (not wrapping a Consumer trait) and is designed
//! specifically for graph-based HTTP servers. It maintains a mapping of request IDs to response
//! senders that is populated by `GraphServer` via `register_request()`.

use crate::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use crate::execution::ExecutionError;
use crate::http_server::types::{HttpServerRequest, HttpServerResponse};
use crate::message::Message;
use crate::traits::{NodeKind, NodeTrait};
use axum::body::Body;
use axum::response::Response;
use futures::StreamExt;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, warn};

/// Configuration for HTTP server consumer node.
#[derive(Debug, Clone)]
pub struct HttpServerConsumerConfig {
  /// Request timeout duration (default: 30 seconds)
  pub request_timeout: std::time::Duration,
  /// Default status code for error responses
  pub default_status: axum::http::StatusCode,
}

impl Default for HttpServerConsumerConfig {
  fn default() -> Self {
    Self {
      request_timeout: std::time::Duration::from_secs(30),
      default_status: axum::http::StatusCode::OK,
    }
  }
}

/// Dedicated graph node type for HTTP server consumer.
///
/// This node:
/// - Receives `Message<HttpServerResponse>` items from input channels
/// - Extracts request_id from the response
/// - Looks up the corresponding response sender
/// - Converts HttpServerResponse to Axum Response and sends it
/// - Implements `NodeTrait` directly (not a wrapper)
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::nodes::HttpServerConsumerNode;
///
/// let node = HttpServerConsumerNode::with_default_config("http_response".to_string());
/// // GraphServer will call register_request() to connect response channels
/// ```
#[derive(Debug)]
pub struct HttpServerConsumerNode {
  /// Node name
  name: String,
  /// Configuration
  config: HttpServerConsumerConfig,
  /// Mapping of request IDs to response senders (set by GraphServer)
  response_senders: Arc<RwLock<HashMap<String, mpsc::Sender<Response<Body>>>>>,
  /// Input port names (single port "in")
  input_port_names: Vec<String>,
}

impl HttpServerConsumerNode {
  /// Creates a new HTTP server consumer node with the given name and configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `config` - Configuration for the consumer
  ///
  /// # Returns
  ///
  /// A new `HttpServerConsumerNode` instance.
  pub fn new(name: String, config: HttpServerConsumerConfig) -> Self {
    Self {
      name,
      config,
      response_senders: Arc::new(RwLock::new(HashMap::new())),
      input_port_names: vec!["in".to_string()],
    }
  }

  /// Creates a new HTTP server consumer node with default configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  ///
  /// # Returns
  ///
  /// A new `HttpServerConsumerNode` instance with default configuration.
  pub fn with_default_config(name: String) -> Self {
    debug!(node = %name, "HttpServerConsumerNode::with_default_config()");
    Self::new(name, HttpServerConsumerConfig::default())
  }

  /// Registers a request and response sender (called by GraphServer).
  ///
  /// # Arguments
  ///
  /// * `request_id` - The unique request ID
  /// * `sender` - Channel sender for sending the response back
  pub(crate) async fn register_request(
    &self,
    request_id: String,
    sender: mpsc::Sender<Response<Body>>,
  ) {
    debug!(
      node = %self.name,
      request_id = %request_id,
      "HttpServerConsumerNode::register_request()"
    );
    let mut senders = self.response_senders.write().await;
    senders.insert(request_id, sender);
  }

  /// Unregisters a request (removes it from the mapping).
  ///
  /// This is called when a request times out or is cancelled.
  ///
  /// # Arguments
  ///
  /// * `request_id` - The request ID to unregister
  pub(crate) async fn unregister_request(&self, request_id: &str) {
    debug!(
      node = %self.name,
      request_id = %request_id,
      "HttpServerConsumerNode::unregister_request()"
    );
    let mut senders = self.response_senders.write().await;
    senders.remove(request_id);
  }

  /// Returns a reference to the configuration.
  #[must_use]
  pub fn config(&self) -> &HttpServerConsumerConfig {
    &self.config
  }

  /// Returns a mutable reference to the configuration.
  pub fn config_mut(&mut self) -> &mut HttpServerConsumerConfig {
    &mut self.config
  }

  /// Converts an `HttpServerResponse` to an Axum `Response<Body>`.
  fn http_response_to_axum(response: &HttpServerResponse) -> Response<Body> {
    let mut axum_response = Response::builder()
      .status(response.status)
      .body(Body::from(response.body.clone()))
      .unwrap_or_else(|_| {
        Response::builder()
          .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
          .body(Body::empty())
          .unwrap()
      });

    // Copy headers
    for (key, value) in response.headers.iter() {
      if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_str().as_bytes()) {
        axum_response
          .headers_mut()
          .insert(header_name, value.clone());
      }
    }

    axum_response
  }
}

impl Clone for HttpServerConsumerNode {
  fn clone(&self) -> Self {
    Self {
      name: self.name.clone(),
      config: self.config.clone(),
      response_senders: Arc::clone(&self.response_senders),
      input_port_names: self.input_port_names.clone(),
    }
  }
}

// DynClone is automatically satisfied by Clone + dyn_clone blanket impls

impl NodeTrait for HttpServerConsumerNode {
  // TODO: This will be refined in task 1.2.x to match actual port configuration
  const INPUT_PORTS: &'static [&'static str] = &["in"];

  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Consumer
  }

  fn input_port_names(&self) -> Vec<String> {
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    vec![] // Consumers have no output ports
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    self.input_port_names.iter().any(|name| name == port_name)
  }

  fn has_output_port(&self, _port_name: &str) -> bool {
    false // Consumers have no output ports
  }

  fn spawn_execution_task(
    &self,
    input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    _output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    _use_shared_memory: bool,
    _arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), ExecutionError>>> {
    let node_name = self.name.clone();
    debug!(
      node = %node_name,
      input_channels = input_channels.len(),
      "HttpServerConsumerNode::spawn_execution_task()"
    );
    let response_senders = Arc::clone(&self.response_senders);
    let _default_status = self.config.default_status;

    let handle = tokio::spawn(async move {
      // Create input stream from channels
      // We need Message<HttpServerResponse> (not just HttpServerResponse) to extract request_id
      // So we manually create streams from receivers
      let receivers: Vec<(String, TypeErasedReceiver)> = input_channels.into_iter().collect();
      let mut merged_stream: Pin<
        Box<dyn futures::Stream<Item = Message<HttpServerResponse>> + Send>,
      > = if receivers.is_empty() {
        Box::pin(futures::stream::empty())
      } else if receivers.len() == 1 {
        let (_port_name, mut receiver) = receivers.into_iter().next().unwrap();
        let node_name_clone = node_name.clone();
        Box::pin(async_stream::stream! {
          while let Some(channel_item) = receiver.recv().await {
            debug!(
              node = %node_name_clone,
              "HttpServerConsumerNode: Received channel item"
            );
            match channel_item {
              ChannelItem::Arc(arc) => {
                let channel_item = ChannelItem::Arc(arc.clone());
                // Try to downcast to Arc<Message<HttpServerResponse>> first
                match channel_item.downcast_message_arc::<HttpServerResponse>() {
                  Ok(msg_arc) => {
                    // Clone the message
                    let message = (*msg_arc).clone();
                    debug!(
                      node = %node_name_clone,
                      request_id = %message.id(),
                      "HttpServerConsumerNode: Received HTTP response"
                    );
                    yield message;
                  }
                  Err(_) => {
                    // Try to downcast to Arc<Message<HttpServerRequest>> and convert to response
                    let channel_item = ChannelItem::Arc(arc.clone());
                    match channel_item.downcast_message_arc::<HttpServerRequest>() {
                      Ok(msg_arc) => {
                        let request_message = (*msg_arc).clone();
                        let request = request_message.payload();
                        debug!(
                          node = %node_name_clone,
                          request_id = %request.request_id,
                          "HttpServerConsumerNode: Received HTTP request, converting to response"
                        );
                        // Convert request to response
                        let response_text = format!("Hello from StreamWeave! You requested: {}", request.path);
                        let response = HttpServerResponse::text_with_request_id(
                          axum::http::StatusCode::OK,
                          &response_text,
                          request.request_id.clone(),
                        );
                        let response_message = Message::with_metadata(
                          response,
                          request_message.id().clone(),
                          request_message.metadata().clone(),
                        );
                        yield response_message;
                      }
                      Err(_) => {
                        warn!(
                          node = %node_name_clone,
                          "Failed to downcast Arc to Message<HttpServerResponse> or Message<HttpServerRequest>, skipping"
                        );
                        continue;
                      }
                    }
                  }
                }
              }
              ChannelItem::SharedMemory(_) => {
                warn!(
                  node = %node_name_clone,
                  "SharedMemory items not yet supported in HttpServerConsumerNode"
                );
                continue;
              }
            }
          }
        })
      } else {
        // Multiple inputs: create streams from all receivers and merge
        let node_name_clone = node_name.clone();
        let streams: Vec<_> = receivers
          .into_iter()
          .map(move |(_port_name, mut receiver)| {
            let node_name_inner = node_name_clone.clone();
            Box::pin(async_stream::stream! {
              while let Some(channel_item) = receiver.recv().await {
                match channel_item {
                  ChannelItem::Arc(arc) => {
                    let channel_item = ChannelItem::Arc(arc.clone());
                    // Try to downcast to Arc<Message<HttpServerResponse>> first
                    match channel_item.downcast_message_arc::<HttpServerResponse>() {
                      Ok(msg_arc) => {
                        let message = (*msg_arc).clone();
                        yield message;
                      }
                      Err(_) => {
                        // Try to downcast to Arc<Message<HttpServerRequest>> and convert to response
                        let channel_item = ChannelItem::Arc(arc.clone());
                        match channel_item.downcast_message_arc::<HttpServerRequest>() {
                          Ok(msg_arc) => {
                            let request_message = (*msg_arc).clone();
                            let request = request_message.payload();
                            // Convert request to response
                            let response_text = format!("Hello from StreamWeave! You requested: {}", request.path);
                            let response = HttpServerResponse::text_with_request_id(
                              axum::http::StatusCode::OK,
                              &response_text,
                              request.request_id.clone(),
                            );
                            let response_message = Message::with_metadata(
                              response,
                              request_message.id().clone(),
                              request_message.metadata().clone(),
                            );
                            yield response_message;
                          }
                          Err(_) => {
                            warn!(
                              node = %node_name_inner,
                              "Failed to downcast Arc to Message<HttpServerResponse> or Message<HttpServerRequest>, skipping"
                            );
                            continue;
                          }
                        }
                      }
                    }
                  }
                  ChannelItem::SharedMemory(_) => {
                    warn!(
                      node = %node_name_inner,
                      "SharedMemory items not yet supported in HttpServerConsumerNode"
                    );
                    continue;
                  }
                }
              }
            })
              as Pin<Box<dyn futures::Stream<Item = Message<HttpServerResponse>> + Send>>
          })
          .collect();
        Box::pin(futures::stream::select_all(streams))
      };

      // Process messages
      loop {
        // Use timeout to periodically check for shutdown
        let message_result = tokio::time::timeout(
          tokio::time::Duration::from_millis(100),
          merged_stream.next(),
        )
        .await;

        let message = match message_result {
          Ok(Some(msg)) => msg,
          Ok(None) => {
            // Stream exhausted
            break;
          }
          Err(_) => {
            // Timeout - check if we should exit (shutdown)
            let paused = *pause_signal.read().await;
            if paused {
              // Pause signal set - during shutdown, exit gracefully
              return Ok(());
            }
            // Not paused, continue waiting for message
            continue;
          }
        };

        // Check pause signal before processing each message
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

        // Extract response and request_id
        let response = message.payload();
        let request_id = response.request_id.clone();

        // Log the outgoing response
        tracing::info!(
          node = %node_name,
          request_id = %request_id,
          status = %response.status,
          "Sending HTTP response"
        );

        // Look up the response sender
        let sender = {
          let senders = response_senders.read().await;
          senders.get(&request_id).cloned()
        };

        match sender {
          Some(sender) => {
            // Convert HttpServerResponse to Axum Response
            let axum_response = Self::http_response_to_axum(response);

            // Send response to client
            if let Err(e) = sender.send(axum_response).await {
              warn!(
                node = %node_name,
                request_id = %request_id,
                error = %e,
                "Failed to send response to client"
              );
            } else {
              tracing::debug!(
                node = %node_name,
                request_id = %request_id,
                "Response sent to client"
              );
            }

            // Remove request from mapping
            let mut senders = response_senders.write().await;
            senders.remove(&request_id);
          }
          None => {
            warn!(
              node = %node_name,
              request_id = %request_id,
              "No sender found for request ID - request may have timed out"
            );
          }
        }
      }

      Ok(())
    });

    Some(handle)
  }
}
