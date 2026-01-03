//! # HTTP Graph Server
//!
//! This module provides a high-level API for creating long-lived graph-based HTTP servers.
//! It integrates Axum HTTP requests with StreamWeave graphs, allowing requests to flow
//! through a graph and responses to be correlated and returned to clients.

#[cfg(feature = "http-server")]
use crate::consumer::HttpResponseCorrelationConsumer;
#[cfg(feature = "http-server")]
use axum::body::Body;
#[cfg(feature = "http-server")]
use axum::extract::Request;
#[cfg(feature = "http-server")]
use axum::http::StatusCode;
#[cfg(feature = "http-server")]
use axum::response::Response;
#[cfg(feature = "http-server")]
use std::sync::Arc;
#[cfg(feature = "http-server")]
use std::time::Duration;
#[cfg(feature = "http-server")]
use streamweave::graph::Graph;
#[cfg(feature = "http-server")]
use streamweave::graph::{ExecutionError, ExecutionState, GraphExecutor};
#[cfg(feature = "http-server")]
use tokio::sync::mpsc;
#[cfg(feature = "http-server")]
use tokio::time::timeout;
#[cfg(feature = "http-server")]
use tracing::{error, warn};

/// Configuration for the HTTP Graph Server.
#[derive(Debug, Clone)]
#[cfg(feature = "http-server")]
pub struct HttpGraphServerConfig {
  /// Request timeout duration (default: 30 seconds)
  pub request_timeout: Duration,
  /// Channel buffer size for request injection (default: 100)
  pub request_channel_buffer: usize,
}

#[cfg(feature = "http-server")]
impl Default for HttpGraphServerConfig {
  fn default() -> Self {
    Self {
      request_timeout: Duration::from_secs(30),
      request_channel_buffer: 100,
    }
  }
}

/// HTTP Graph Server that maintains a long-lived graph for processing HTTP requests.
///
/// This server:
/// - Maintains a single long-lived graph executor
/// - Accepts HTTP requests via Axum and injects them into the graph
/// - Correlates responses with requests and returns them to clients
/// - Handles request timeouts and errors
#[cfg(feature = "http-server")]
pub struct HttpGraphServer {
  /// Graph executor for running the graph
  executor: Arc<tokio::sync::RwLock<GraphExecutor>>,
  /// Channel sender for injecting requests into the graph
  request_sender: mpsc::Sender<Request>,
  /// Response correlation consumer for matching responses to requests
  response_consumer: Arc<HttpResponseCorrelationConsumer>,
  /// Server configuration
  config: HttpGraphServerConfig,
}

#[cfg(feature = "http-server")]
impl HttpGraphServer {
  /// Creates a new HTTP Graph Server from a constructed graph.
  ///
  /// # Arguments
  ///
  /// * `graph` - The graph to execute (must contain LongLivedHttpRequestProducer and HttpResponseCorrelationConsumer)
  /// * `config` - Server configuration
  ///
  /// # Returns
  ///
  /// A new `HttpGraphServer` instance and a channel receiver for requests.
  /// The receiver should be passed to the `LongLivedHttpRequestProducer` in the graph.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be executed or if required nodes are missing.
  pub async fn new(
    graph: Graph,
    config: HttpGraphServerConfig,
  ) -> Result<(Self, mpsc::Receiver<Request>), ExecutionError> {
    // Create channel for injecting requests
    let (request_sender, request_receiver) = mpsc::channel(config.request_channel_buffer);

    // Create response correlation consumer
    let response_consumer = Arc::new(HttpResponseCorrelationConsumer::with_timeout(
      config.request_timeout,
    ));

    // Create graph executor
    let executor = GraphExecutor::new(graph);

    let server = Self {
      executor: Arc::new(tokio::sync::RwLock::new(executor)),
      request_sender,
      response_consumer,
      config,
    };

    Ok((server, request_receiver))
  }

  /// Starts the graph executor.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be started.
  pub async fn start(&self) -> Result<(), ExecutionError> {
    let mut executor = self.executor.write().await;
    executor.start().await
  }

  /// Stops the graph executor.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be stopped.
  pub async fn stop(&self) -> Result<(), ExecutionError> {
    let mut executor = self.executor.write().await;
    executor.stop().await
  }

  /// Pauses the graph executor.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be paused.
  pub async fn pause(&self) -> Result<(), ExecutionError> {
    let mut executor = self.executor.write().await;
    executor.pause().await
  }

  /// Resumes the graph executor.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be resumed.
  pub async fn resume(&self) -> Result<(), ExecutionError> {
    let mut executor = self.executor.write().await;
    executor.resume().await
  }

  /// Gets the current execution state.
  pub async fn state(&self) -> ExecutionState {
    let executor = self.executor.read().await;
    executor.state()
  }

  /// Handles an HTTP request by injecting it into the graph and waiting for a response.
  ///
  /// This method:
  /// 1. Extracts the request ID from the HTTP request
  /// 2. Creates a response channel for this request
  /// 3. Registers the request with the response correlation consumer
  /// 4. Injects the request into the graph
  /// 5. Waits for the response (with timeout)
  /// 6. Returns the response to the client
  ///
  /// # Arguments
  ///
  /// * `request` - The Axum HTTP request
  ///
  /// # Returns
  ///
  /// The HTTP response, or a timeout/error response if something goes wrong.
  pub async fn handle_request(&self, mut request: Request) -> Response<Body> {
    // Generate a unique request ID
    let request_id = uuid::Uuid::new_v4().to_string();

    // Store the request_id in request extensions so the producer can use it
    use crate::types::RequestIdExtension;
    request
      .extensions_mut()
      .insert(RequestIdExtension(request_id.clone()));

    // Create a channel for receiving the response
    let (response_sender, mut response_receiver) = mpsc::channel(1);

    // Register the request with the response correlation consumer
    self
      .response_consumer
      .register_request(request_id.clone(), response_sender)
      .await;

    // Inject the request into the graph
    if let Err(e) = self.request_sender.send(request).await {
      error!(
        request_id = %request_id,
        error = %e,
        "Failed to inject request into graph"
      );
      return Self::create_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to process request",
      );
    }

    // Wait for the response with timeout
    match timeout(self.config.request_timeout, response_receiver.recv()).await {
      Ok(Some(response)) => response,
      Ok(None) => {
        warn!(
          request_id = %request_id,
          "Response channel closed before response received"
        );
        Self::create_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Response channel closed")
      }
      Err(_) => {
        warn!(
          request_id = %request_id,
          timeout_secs = self.config.request_timeout.as_secs(),
          "Request timed out waiting for response"
        );
        Self::create_error_response(StatusCode::GATEWAY_TIMEOUT, "Request timed out")
      }
    }
  }

  /// Creates an error response.
  fn create_error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
      .status(status)
      .header("Content-Type", "application/json")
      .body(Body::from(format!(
        r#"{{"error":"{}"}}"#,
        message.replace('"', "\\\"")
      )))
      .unwrap_or_else(|_| {
        Response::builder()
          .status(StatusCode::INTERNAL_SERVER_ERROR)
          .body(Body::from("Internal server error"))
          .unwrap()
      })
  }

  /// Creates an Axum handler function that can be used with Axum routes.
  ///
  /// # Returns
  ///
  /// An Axum handler function that processes requests through the graph.
  pub fn create_handler(
    self,
  ) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
  + Send
  + Sync
  + Clone
  + 'static {
    let server = Arc::new(self);
    move |request: Request| {
      let server = Arc::clone(&server);
      Box::pin(async move { server.handle_request(request).await })
    }
  }
}

#[cfg(feature = "http-server")]
impl Clone for HttpGraphServer {
  fn clone(&self) -> Self {
    Self {
      executor: Arc::clone(&self.executor),
      request_sender: self.request_sender.clone(),
      response_consumer: Arc::clone(&self.response_consumer),
      config: self.config.clone(),
    }
  }
}
