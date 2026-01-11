//! # HTTP Path Router Node
//!
//! Dedicated graph node type for HTTP path-based routing that routes `Message<HttpServerRequest>`
//! items to different output ports based on path patterns.
//!
//! This node implements `NodeTrait` directly (not wrapping a Transformer trait) and is designed
//! specifically for graph-based HTTP servers. It uses `PathBasedRouterTransformer` internally
//! to perform the routing logic.

use crate::Transformer;
use crate::graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use crate::graph::execution::ExecutionError;
use crate::graph::http_server::nodes::path_router_transformer::{
  PathBasedRouterTransformer, PathRouterConfig,
};
use crate::graph::http_server::types::HttpServerRequest;
use crate::graph::traits::{NodeKind, NodeTrait};
use crate::message::Message;
use async_stream::stream;
use futures::StreamExt;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Configuration for HTTP path router node.
#[derive(Debug, Clone, Default)]
pub struct HttpPathRouterConfig {
  /// Router configuration
  pub router_config: PathRouterConfig,
}

/// Dedicated graph node type for HTTP path-based routing.
///
/// This node:
/// - Receives `Message<HttpServerRequest>` items from input channels
/// - Routes them to different output ports based on path patterns
/// - Supports up to 5 output ports (out_0 through out_4)
/// - Implements `NodeTrait` directly (not a wrapper)
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::graph::http_server::nodes::HttpPathRouterNode;
///
/// let mut node = HttpPathRouterNode::with_default_config("path_router".to_string());
/// node.add_route("/api/rest/*".to_string(), 0);
/// node.add_route("/api/graphql".to_string(), 1);
/// ```
#[derive(Debug)]
pub struct HttpPathRouterNode {
  /// Node name
  name: String,
  /// Configuration
  config: HttpPathRouterConfig,
  /// Internal transformer (used for routing logic)
  transformer: Arc<Mutex<PathBasedRouterTransformer>>,
  /// Input port names (single port "in")
  input_port_names: Vec<String>,
  /// Output port names (5 ports: out_0 through out_4)
  output_port_names: Vec<String>,
}

impl HttpPathRouterNode {
  /// Creates a new HTTP path router node with the given name and configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `config` - Configuration for the router
  ///
  /// # Returns
  ///
  /// A new `HttpPathRouterNode` instance.
  pub fn new(name: String, config: HttpPathRouterConfig) -> Self {
    let transformer = PathBasedRouterTransformer::new(config.router_config.clone());
    Self {
      name,
      config,
      transformer: Arc::new(Mutex::new(transformer)),
      input_port_names: vec!["in".to_string()],
      output_port_names: vec![
        "out_0".to_string(),
        "out_1".to_string(),
        "out_2".to_string(),
        "out_3".to_string(),
        "out_4".to_string(),
      ],
    }
  }

  /// Creates a new HTTP path router node with default configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  ///
  /// # Returns
  ///
  /// A new `HttpPathRouterNode` instance with default configuration.
  pub fn with_default_config(name: String) -> Self {
    debug!(node = %name, "HttpPathRouterNode::with_default_config()");
    Self::new(name, HttpPathRouterConfig::default())
  }

  /// Adds a route pattern to the router.
  ///
  /// # Arguments
  ///
  /// * `pattern` - The path pattern to match (supports wildcards like `/api/*`)
  /// * `port` - The output port index for this route (0-4)
  pub fn add_route(&mut self, pattern: String, port: usize) {
    debug!(
      node = %self.name,
      pattern = %pattern,
      port = port,
      "HttpPathRouterNode::add_route()"
    );
    let mut transformer = self.transformer.try_lock().unwrap();
    transformer.add_route(pattern, port);
  }

  /// Sets the default port for unmatched requests.
  ///
  /// # Arguments
  ///
  /// * `port` - The port index for unmatched requests, or `None` to drop them
  pub fn set_default_port(&mut self, port: Option<usize>) {
    debug!(
      node = %self.name,
      default_port = ?port,
      "HttpPathRouterNode::set_default_port()"
    );
    let mut transformer = self.transformer.try_lock().unwrap();
    transformer.set_default_port(port);
  }

  /// Returns a reference to the configuration.
  #[must_use]
  pub fn config(&self) -> &HttpPathRouterConfig {
    &self.config
  }

  /// Returns a mutable reference to the configuration.
  pub fn config_mut(&mut self) -> &mut HttpPathRouterConfig {
    &mut self.config
  }
}

impl Clone for HttpPathRouterNode {
  fn clone(&self) -> Self {
    Self {
      name: self.name.clone(),
      config: self.config.clone(),
      transformer: Arc::clone(&self.transformer),
      input_port_names: self.input_port_names.clone(),
      output_port_names: self.output_port_names.clone(),
    }
  }
}

// DynClone is automatically satisfied by Clone + dyn_clone blanket impls

impl NodeTrait for HttpPathRouterNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Transformer
  }

  fn input_port_names(&self) -> Vec<String> {
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    self.output_port_names.clone()
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    self.input_port_names.iter().any(|name| name == port_name)
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    self.output_port_names.iter().any(|name| name == port_name)
  }

  fn spawn_execution_task(
    &self,
    input_channels: HashMap<String, TypeErasedReceiver>,
    output_channels: HashMap<String, TypeErasedSender>,
    pause_signal: Arc<tokio::sync::RwLock<bool>>,
    _use_shared_memory: bool,
    _arc_pool: Option<Arc<crate::graph::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), ExecutionError>>> {
    let node_name = self.name.clone();
    debug!(
      node = %node_name,
      input_channels = input_channels.len(),
      output_channels = output_channels.len(),
      "HttpPathRouterNode::spawn_execution_task()"
    );
    let transformer = Arc::clone(&self.transformer);

    let handle = tokio::spawn(async move {
      // Create input stream from channels
      // We need Message<HttpServerRequest>
      let receivers: Vec<(String, TypeErasedReceiver)> = input_channels.into_iter().collect();
      let input_stream: Pin<Box<dyn futures::Stream<Item = Message<HttpServerRequest>> + Send>> =
        if receivers.is_empty() {
          Box::pin(futures::stream::empty())
        } else if receivers.len() == 1 {
          let (_port_name, mut receiver) = receivers.into_iter().next().unwrap();
          let node_name_clone = node_name.clone();
          Box::pin(stream! {
            while let Some(channel_item) = receiver.recv().await {
              tracing::info!(
                node = %node_name_clone,
                "Path router received channel item"
              );
              match channel_item {
                ChannelItem::Arc(arc) => {
                  let channel_item = ChannelItem::Arc(arc.clone());
                  match channel_item.downcast_message_arc::<HttpServerRequest>() {
                    Ok(msg_arc) => {
                      let message = (*msg_arc).clone();
                      let request = message.payload();
                      tracing::info!(
                        node = %node_name_clone,
                        request_id = %request.request_id,
                        path = %request.path,
                        "Path router received HTTP request"
                      );
                      yield message;
                    }
                    Err(_) => {
                      warn!(
                        node = %node_name_clone,
                        "Failed to downcast Arc to Message<HttpServerRequest>, skipping"
                      );
                      continue;
                    }
                  }
                }
                ChannelItem::SharedMemory(_) => {
                  warn!(
                    node = %node_name_clone,
                    "SharedMemory items not yet supported in HttpPathRouterNode"
                  );
                  continue;
                }
              }
            }
          })
        } else {
          // Multiple inputs: merge streams
          let node_name_clone = node_name.clone();
          let streams: Vec<_> = receivers
            .into_iter()
            .map(move |(_port_name, mut receiver)| {
              let node_name_inner = node_name_clone.clone();
              Box::pin(stream! {
                while let Some(channel_item) = receiver.recv().await {
                  tracing::info!(
                    node = %node_name_inner,
                    "Path router received channel item (multi-input)"
                  );
                  match channel_item {
                    ChannelItem::Arc(arc) => {
                      let channel_item = ChannelItem::Arc(arc.clone());
                      match channel_item.downcast_message_arc::<HttpServerRequest>() {
                        Ok(msg_arc) => {
                          let message = (*msg_arc).clone();
                          let request = message.payload();
                          tracing::info!(
                            node = %node_name_inner,
                            request_id = %request.request_id,
                            path = %request.path,
                            "Path router received HTTP request (multi-input)"
                          );
                          yield message;
                        }
                        Err(_) => {
                          warn!(
                            node = %node_name_inner,
                            "Failed to downcast Arc to Message<HttpServerRequest>, skipping"
                          );
                          continue;
                        }
                      }
                    }
                    ChannelItem::SharedMemory(_) => {
                      warn!(
                        node = %node_name_inner,
                        "SharedMemory items not yet supported in HttpPathRouterNode"
                      );
                      continue;
                    }
                  }
                }
              })
                as Pin<Box<dyn futures::Stream<Item = Message<HttpServerRequest>> + Send>>
            })
            .collect();
          Box::pin(futures::stream::select_all(streams))
        };

      // Use transformer to route messages
      let mut transformer_guard = transformer.lock().await;
      let output_stream = transformer_guard.transform(input_stream).await;
      drop(transformer_guard);

      // Process routed messages and send to appropriate output ports
      let mut output_stream = std::pin::pin!(output_stream);
      loop {
        // Use timeout to periodically check for shutdown
        let item_result = tokio::time::timeout(
          tokio::time::Duration::from_millis(100),
          output_stream.next(),
        )
        .await;

        let output_tuple = match item_result {
          Ok(Some(tuple)) => tuple,
          Ok(None) => {
            // Stream exhausted
            break;
          }
          Err(_) => {
            // Timeout - check if we should exit (shutdown)
            let paused = *pause_signal.read().await;
            if paused {
              return Ok(());
            }
            continue;
          }
        };

        // Check pause signal
        let pause_check_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
            while *pause_signal.read().await {
              tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
          })
          .await;

        if pause_check_result.is_err() && *pause_signal.read().await {
          return Ok(());
        }

        // Send to appropriate output ports
        // Output tuple is (Option<Message>, Option<Message>, Option<Message>, Option<Message>, Option<Message>)
        let ports = [
          ("out_0", &output_tuple.0),
          ("out_1", &output_tuple.1),
          ("out_2", &output_tuple.2),
          ("out_3", &output_tuple.3),
          ("out_4", &output_tuple.4),
        ];

        for (port_name, opt_message) in ports.iter() {
          if let Some(message) = opt_message {
            // Log routing decision
            let request = message.payload();
            tracing::info!(
              node = %node_name,
              request_id = %request.request_id,
              path = %request.path,
              output_port = %port_name,
              "Routing HTTP request"
            );

            if let Some(sender) = output_channels.get(*port_name) {
              // Wrap in Arc and send
              let message_arc = Arc::new(message.clone());
              let arc_any: Arc<dyn Any + Send + Sync> = unsafe {
                Arc::from_raw(Arc::into_raw(message_arc) as *const (dyn Any + Send + Sync))
              };
              if sender.send(ChannelItem::Arc(arc_any)).await.is_err() {
                let paused = *pause_signal.read().await;
                if paused {
                  return Ok(());
                }
                warn!(
                  node = %node_name,
                  port = %port_name,
                  "Output channel receiver dropped"
                );
              }
            }
          }
        }
      }

      Ok(())
    });

    Some(handle)
  }
}
