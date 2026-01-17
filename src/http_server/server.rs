//! # Graph Server
//!
//! Integration between StreamWeave graphs and Axum HTTP servers.
//!
//! This module provides `GraphServer` which discovers HTTP server nodes in a graph,
//! connects them to Axum, and provides handlers for serving HTTP requests.

use crate;
use crate::execution::{ExecutionError, GraphExecutor};
use crate::http_server::nodes::consumer_node::HttpServerConsumerNode;
use crate::http_server::nodes::producer_node::HttpServerProducerNode;
use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::Response;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

/// Configuration for the Graph Server.
#[derive(Debug, Clone)]
pub struct GraphServerConfig {
  /// Request timeout duration (default: 30 seconds)
  pub request_timeout: Duration,
  /// Channel buffer size for request injection (default: 100)
  pub request_channel_buffer: usize,
}

impl Default for GraphServerConfig {
  fn default() -> Self {
    Self {
      request_timeout: Duration::from_secs(30),
      request_channel_buffer: 100,
    }
  }
}

/// Graph Server that integrates a StreamWeave graph with an Axum HTTP server.
///
/// This server:
/// - Discovers `HttpServerProducerNode` and `HttpServerConsumerNode` in the graph
/// - Creates channels and connects them to the nodes
/// - Starts the graph executor
/// - Provides Axum handlers for HTTP requests
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::{Graph, GraphBuilder};
/// use streamweave::http_server::{GraphServer, HttpServerProducerNode, HttpServerConsumerNode};
///
/// // Build a graph with HTTP server nodes
/// let graph = GraphBuilder::new()
///     .node(HttpServerProducerNode::with_default_config("http_server".to_string()))
///     .node(HttpServerConsumerNode::with_default_config("http_response".to_string()))
///     .connect_by_name("http_server", "http_response")?
///     .build();
///
/// // Create and serve the graph
/// let server = GraphServer::from_graph(graph, GraphServerConfig::default()).await?;
/// server.serve("0.0.0.0:3000".parse().unwrap()).await?;
/// ```
pub struct GraphServer {
  /// Graph executor for running the graph
  executor: Arc<tokio::sync::RwLock<GraphExecutor>>,
  /// Channel sender for injecting requests into the graph
  request_sender: mpsc::Sender<Request>,
  /// Reference to the producer node (for request injection)
  producer_node: Arc<tokio::sync::RwLock<Option<Arc<HttpServerProducerNode>>>>,
  /// Reference to the consumer node (for response correlation)
  consumer_node: Arc<tokio::sync::RwLock<Option<Arc<HttpServerConsumerNode>>>>,
  /// Server configuration
  config: GraphServerConfig,
}

impl GraphServer {
  /// Creates a new Graph Server from a constructed graph.
  ///
  /// This method:
  /// 1. Extracts `HttpServerProducerNode` and `HttpServerConsumerNode` from the graph by name
  /// 2. Creates channels for request injection and response correlation
  /// 3. Connects the channels to the nodes
  /// 4. Creates a graph executor
  ///
  /// # Arguments
  ///
  /// * `graph` - The graph to execute (must contain HttpServerProducerNode and HttpServerConsumerNode)
  /// * `producer_node_name` - The name of the HttpServerProducerNode in the graph
  /// * `consumer_node_name` - The name of the HttpServerConsumerNode in the graph
  /// * `config` - Server configuration
  ///
  /// # Returns
  ///
  /// A new `GraphServer` instance.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The graph cannot be executed
  /// - Required nodes are missing or have wrong types
  /// - Channels cannot be created or connected
  pub async fn from_graph_with_node_names(
    graph: Graph,
    producer_node_name: String,
    consumer_node_name: String,
    config: GraphServerConfig,
  ) -> Result<Self, ExecutionError> {
    trace!(
      producer_node = %producer_node_name,
      consumer_node = %consumer_node_name,
      "GraphServer::from_graph_with_node_names"
    );
    // Get nodes from graph and verify they exist
    let producer_node_trait = graph.get_node(&producer_node_name).ok_or_else(|| {
      ExecutionError::Other(format!(
        "Producer node '{}' not found in graph",
        producer_node_name
      ))
    })?;

    let consumer_node_trait = graph.get_node(&consumer_node_name).ok_or_else(|| {
      ExecutionError::Other(format!(
        "Consumer node '{}' not found in graph",
        consumer_node_name
      ))
    })?;

    // Verify node kinds
    if producer_node_trait.node_kind() != crate::traits::NodeKind::Producer {
      return Err(ExecutionError::Other(format!(
        "Node '{}' is not a Producer (found {:?})",
        producer_node_name,
        producer_node_trait.node_kind()
      )));
    }

    if consumer_node_trait.node_kind() != crate::traits::NodeKind::Consumer {
      return Err(ExecutionError::Other(format!(
        "Node '{}' is not a Consumer (found {:?})",
        consumer_node_name,
        consumer_node_trait.node_kind()
      )));
    }

    // Downcast and clone nodes
    // Since NodeTrait extends Any, we can use downcast_ref
    // Cast &dyn NodeTrait to &dyn Any, then use downcast_ref
    let producer_node: Arc<HttpServerProducerNode> = {
      let any_node = producer_node_trait as &dyn std::any::Any;
      match any_node.downcast_ref::<HttpServerProducerNode>() {
        Some(node) => Arc::new(node.clone()),
        None => {
          return Err(ExecutionError::Other(format!(
            "Node '{}' is not an HttpServerProducerNode (found: {})",
            producer_node_name,
            std::any::type_name_of_val(producer_node_trait)
          )));
        }
      }
    };

    let consumer_node: Arc<HttpServerConsumerNode> = {
      let any_node = consumer_node_trait as &dyn std::any::Any;
      match any_node.downcast_ref::<HttpServerConsumerNode>() {
        Some(node) => Arc::new(node.clone()),
        None => {
          return Err(ExecutionError::Other(format!(
            "Node '{}' is not an HttpServerConsumerNode (found: {})",
            consumer_node_name,
            std::any::type_name_of_val(consumer_node_trait)
          )));
        }
      }
    };

    // Create channel for injecting requests
    debug!(
      buffer_size = config.request_channel_buffer,
      "Creating request channel"
    );
    let (request_sender, request_receiver) = mpsc::channel(config.request_channel_buffer);

    // Set the request receiver on the producer node
    // Since the node uses Arc<Mutex<Option<Receiver>>>, setting it on our clone
    // will also affect the node in the graph (they share the same Arc)
    debug!(
      producer_node = %producer_node_name,
      "Setting request receiver on producer node"
    );
    producer_node.set_request_receiver(request_receiver).await;

    // Create graph executor
    debug!("Creating GraphExecutor");
    let executor = GraphExecutor::new(graph);

    let server = Self {
      executor: Arc::new(tokio::sync::RwLock::new(executor)),
      request_sender,
      producer_node: Arc::new(tokio::sync::RwLock::new(Some(producer_node))),
      consumer_node: Arc::new(tokio::sync::RwLock::new(Some(consumer_node))),
      config,
    };

    Ok(server)
  }

  /// Creates a new Graph Server from a graph with automatic node discovery.
  ///
  /// This is a convenience method that searches for HttpServerProducerNode and HttpServerConsumerNode
  /// in the graph. It requires exactly one of each node type.
  ///
  /// # Arguments
  ///
  /// * `graph` - The graph to execute
  /// * `config` - Server configuration
  ///
  /// # Returns
  ///
  /// A new `GraphServer` instance.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The graph cannot be executed
  /// - Required nodes are missing or there are multiple nodes of the same type
  /// - Channels cannot be created or connected
  pub async fn from_graph(
    _graph: Graph,
    _config: GraphServerConfig,
  ) -> Result<Self, ExecutionError> {
    // For now, we'll use a simplified approach
    // TODO: Implement proper node discovery and extraction
    // This requires changes to Graph to support node extraction
    Err(ExecutionError::Other(
      "GraphServer::from_graph not yet implemented - use from_graph_with_node_names or pass nodes directly".to_string()
    ))
  }

  /// Starts the graph executor.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be started.
  pub async fn start(&self) -> Result<(), ExecutionError> {
    trace!("GraphServer::start()");
    info!("Starting graph executor...");
    let mut executor = self.executor.write().await;
    let result = executor.start().await;
    match &result {
      Ok(()) => info!("Graph executor started successfully"),
      Err(e) => error!(error = %e, "Failed to start graph executor"),
    }
    result
  }

  /// Stops the graph executor.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be stopped.
  pub async fn stop(&self) -> Result<(), ExecutionError> {
    trace!("GraphServer::stop()");
    let mut executor = self.executor.write().await;
    executor.stop().await
  }

  /// Handles an HTTP request by injecting it into the graph and waiting for a response.
  ///
  /// This method:
  /// 1. Extracts or generates a request ID
  /// 2. Creates a response channel for this request
  /// 3. Registers the request with the consumer node
  /// 4. Injects the request into the graph via the producer node
  /// 5. Waits for the response (with timeout)
  /// 6. Returns the response to the client
  ///
  /// # Arguments
  ///
  /// * `request` - The Axum HTTP request
  ///
  /// # Returns
  ///
  /// An Axum HTTP response, or an error response if something goes wrong.
  pub async fn handle(&self, request: Request) -> Response<Body> {
    trace!("GraphServer::handle()");
    // Generate request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    debug!(request_id = %request_id, "Generated request ID");

    // Create response channel
    let (response_sender, mut response_receiver) = mpsc::channel(1);

    // Register request with consumer node
    debug!(request_id = %request_id, "Registering request with consumer node");
    if let Some(consumer) = self.consumer_node.read().await.as_ref() {
      consumer
        .register_request(request_id.clone(), response_sender)
        .await;
    } else {
      error!("HttpServerConsumerNode not found - cannot handle request");
      return Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Server configuration error"))
        .unwrap();
    }

    // Inject request into graph
    // Add request ID to request extensions so producer can use it
    debug!(request_id = %request_id, "Injecting request into graph");
    let mut request_with_id = request;
    request_with_id
      .extensions_mut()
      .insert(crate::http_server::types::RequestIdExtension(
        request_id.clone(),
      ));
    if let Err(e) = self.request_sender.send(request_with_id).await {
      error!(error = %e, "Failed to inject request into graph");
      return Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Failed to process request"))
        .unwrap();
    }

    // Wait for response with timeout
    match timeout(self.config.request_timeout, response_receiver.recv()).await {
      Ok(Some(response)) => response,
      Ok(None) => {
        warn!(request_id = %request_id, "Response channel closed before response received");
        Response::builder()
          .status(StatusCode::INTERNAL_SERVER_ERROR)
          .body(Body::from("Request processing failed"))
          .unwrap()
      }
      Err(_) => {
        warn!(request_id = %request_id, "Request timed out");
        // Unregister request
        if let Some(consumer) = self.consumer_node.read().await.as_ref() {
          consumer.unregister_request(&request_id).await;
        }
        Response::builder()
          .status(StatusCode::REQUEST_TIMEOUT)
          .body(Body::from("Request timeout"))
          .unwrap()
      }
    }
  }

  /// Returns an Axum handler function for this server.
  ///
  /// This can be used to integrate the server into a custom Axum router.
  ///
  /// # Returns
  ///
  /// A handler function that takes a `Request` and returns a `Response<Body>`.
  pub fn handler(
    &self,
  ) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
  + Clone {
    let server = Arc::new(self.clone());
    move |request: Request| {
      let server = Arc::clone(&server);
      Box::pin(async move { server.handle(request).await })
    }
  }

  /// Serves the graph on the given address.
  ///
  /// This method:
  /// 1. Starts the graph executor
  /// 2. Creates an Axum router with the handler
  /// 3. Serves the router on the given address
  ///
  /// # Arguments
  ///
  /// * `addr` - The address to bind to (e.g., "0.0.0.0:3000")
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The graph cannot be started
  /// - The server cannot bind to the address
  /// - The server fails to serve
  pub async fn serve(&self, addr: std::net::SocketAddr) -> Result<(), ExecutionError> {
    trace!(addr = %addr, "GraphServer::serve()");
    // Start the graph executor
    info!("Starting graph executor before serving...");
    self.start().await?;
    info!("Graph executor started, starting HTTP server...");

    // Create handler - clone self into Arc for 'static lifetime
    let server = Arc::new(self.clone());
    let handler = move |request: Request| {
      let server = server.clone();
      async move { server.handle(request).await }
    };
    let router = Router::new().route("/", axum::routing::any(handler));

    // Serve
    let listener = tokio::net::TcpListener::bind(addr)
      .await
      .map_err(|e| ExecutionError::Other(format!("Failed to bind to address {}: {}", addr, e)))?;

    axum::serve(listener, router)
      .await
      .map_err(|e| ExecutionError::Other(format!("Server error: {}", e)))?;

    Ok(())
  }
}

impl Clone for GraphServer {
  fn clone(&self) -> Self {
    trace!("GraphServer::clone()");
    Self {
      executor: Arc::clone(&self.executor),
      request_sender: self.request_sender.clone(),
      producer_node: Arc::clone(&self.producer_node),
      consumer_node: Arc::clone(&self.consumer_node),
      config: self.config.clone(),
    }
  }
}
