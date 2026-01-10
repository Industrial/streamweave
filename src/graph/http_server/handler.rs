//! # Axum Route Handler Integration
//!
//! This module provides Axum route handlers that integrate HTTP requests/responses
//! with StreamWeave pipelines, allowing users to define REST endpoints using
//! StreamWeave for processing.

use crate::Producer;
use crate::graph::http_server::error::{is_development_mode, map_to_http_error};
use crate::graph::http_server::graph_server::HttpGraphServer;
use crate::graph::http_server::nodes::consumer::HttpResponseConsumer;
use crate::graph::http_server::nodes::producer::{HttpRequestProducer, HttpRequestProducerConfig};
use crate::graph::http_server::types::{HttpServerRequest, HttpServerResponse};
use crate::pipeline::PipelineBuilder;
use axum::http::StatusCode;
use axum::{body::Body, extract::Request, response::Response};
use chrono::Utc;
use futures::StreamExt;
use std::sync::Arc;
use tracing::{error, warn};

/// Creates a simple handler that processes requests with a transformer function.
///
/// This is a convenience function for simple cases where you just want to transform
/// the request and return a response, without explicitly building a full pipeline.
///
/// ## Example
///
/// ```rust,no_run
/// use crate::http_server::create_simple_handler;
/// use crate::http_server::{HttpServerRequest, HttpServerResponse};
/// use axum::http::StatusCode;
/// use axum::{Router, routing::get};
///
/// let app = Router::new()
///     .route("/api/echo", get(create_simple_handler(|req: HttpServerRequest| {
///         HttpServerResponse::text(StatusCode::OK, &format!("Echo: {}", req.path))
///     })));
/// ```
///
/// ## Arguments
///
/// * `transform_fn` - A function that transforms an HttpServerRequest into an HttpServerResponse
///
/// ## Returns
///
/// An Axum handler function
pub fn create_simple_handler<F>(
  transform_fn: F,
) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
where
  F: Fn(HttpServerRequest) -> HttpServerResponse + Send + Sync + Clone + 'static,
{
  let transform_fn = Arc::new(transform_fn);
  move |request: Request| {
    let transform_fn = transform_fn.clone();
    Box::pin(async move { handle_simple_request(request, &*transform_fn).await })
  }
}

/// Internal function that handles a simple request transformation.
async fn handle_simple_request<F>(axum_request: Request, transform_fn: &F) -> Response<Body>
where
  F: Fn(HttpServerRequest) -> HttpServerResponse,
{
  // Create HTTP request producer
  let producer = HttpRequestProducer::from_axum_request(
    axum_request,
    HttpRequestProducerConfig::default()
      .with_extract_body(true)
      .with_parse_json(true),
  )
  .await;

  // Get the request from the producer
  let mut request_stream = {
    let mut producer = producer;
    <HttpRequestProducer as Producer>::produce(&mut producer)
  };
  if let Some(request) = request_stream.next().await {
    // Transform the request
    let response = transform_fn(request);
    response.to_axum_response()
  } else {
    // No request available
    warn!("No request available from producer");
    let error_response = map_to_http_error(
      &crate::error::StreamError::<HttpServerRequest>::new(
        Box::new(std::io::Error::other("Failed to extract request")),
        crate::error::ErrorContext {
          timestamp: Utc::now(),
          item: None,
          component_name: "http_request_producer".to_string(),
          component_type: "HttpRequestProducer".to_string(),
        },
        crate::error::ComponentInfo {
          name: "http_request_producer".to_string(),
          type_name: "HttpRequestProducer".to_string(),
        },
      ),
      is_development_mode(),
    );
    HttpServerResponse::json(StatusCode::BAD_REQUEST, &error_response)
      .unwrap_or_else(|_| {
        HttpServerResponse::error(StatusCode::BAD_REQUEST, "Failed to extract request")
      })
      .to_axum_response()
  }
}

/// Creates a handler that processes requests through a full StreamWeave pipeline.
///
/// This function builds a complete pipeline with HttpRequestProducer, a transformer,
/// and HttpResponseConsumer, then runs it. The transformer should convert HttpServerRequest
/// to HttpServerResponse.
///
/// ## Example
///
/// ```rust,no_run
/// use crate::http_server::create_pipeline_handler;
/// use crate::transformers::MapTransformer;
/// use crate::http_server::{HttpServerRequest, HttpServerResponse};
/// use axum::http::StatusCode;
/// use axum::{Router, routing::get};
///
/// let app = Router::new()
///     .route("/api/process", get(create_pipeline_handler(|| {
///         MapTransformer::new(|req: HttpServerRequest| {
///             HttpServerResponse::text(StatusCode::OK, "Processed")
///         })
///     })));
/// ```
///
/// ## Arguments
///
/// * `transformer_fn` - A function that returns a transformer for the pipeline
///
/// ## Returns
///
/// An Axum handler function
pub fn create_pipeline_handler<T>(
  transformer_fn: impl Fn() -> T + Send + Sync + Clone + 'static,
) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
where
  T: crate::Transformer + Send + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  HttpRequestProducer: crate::Producer,
  <HttpRequestProducer as crate::Output>::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpRequestProducer as crate::Output>::OutputStream: 'static,
  T::InputStream: From<<HttpRequestProducer as crate::Output>::OutputStream>,
  HttpResponseConsumer: crate::Consumer,
  <HttpResponseConsumer as crate::Input>::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpResponseConsumer as crate::Input>::InputStream: From<T::OutputStream>,
{
  let transformer_fn: Arc<dyn Fn() -> T + Send + Sync> = Arc::new(transformer_fn);
  move |request: Request| {
    let transformer_fn = Arc::clone(&transformer_fn);
    Box::pin(
      async move { handle_request_with_pipeline(request, move || (*transformer_fn)()).await },
    )
  }
}

/// Internal function that handles a request through a pipeline.
async fn handle_request_with_pipeline<T, F>(
  axum_request: Request,
  transformer_fn: F,
) -> Response<Body>
where
  F: FnOnce() -> T + Send + 'static,
  T: crate::Transformer + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  HttpRequestProducer: crate::Producer,
  <HttpRequestProducer as crate::Output>::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpRequestProducer as crate::Output>::OutputStream: 'static,
  T::InputStream: From<<HttpRequestProducer as crate::Output>::OutputStream>,
  HttpResponseConsumer: crate::Consumer,
  <HttpResponseConsumer as crate::Input>::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpResponseConsumer as crate::Input>::InputStream: From<T::OutputStream>,
{
  // Create HTTP request producer from Axum request
  let producer = HttpRequestProducer::from_axum_request(
    axum_request,
    HttpRequestProducerConfig::default()
      .with_extract_body(true)
      .with_parse_json(true),
  )
  .await;

  // Create transformer
  let transformer = transformer_fn();

  // Create consumer
  let consumer = HttpResponseConsumer::new();

  // Build and run pipeline
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  // Run the pipeline
  match pipeline.run().await {
    Ok((_, mut consumer)) => {
      // Check if streaming is enabled
      if consumer.http_config().stream_response {
        // For streaming, we need to get the stream from the pipeline output
        // This is a simplified version - in practice, you'd want to stream directly
        // from the transformer output without collecting in the consumer
        consumer.get_response().await
      } else {
        // Get the response from the consumer (non-streaming)
        consumer.get_response().await
      }
    }
    Err(e) => {
      error!(error = ?e, "Pipeline execution failed");
      let error_response = map_to_http_error(
        &crate::error::StreamError::<HttpServerResponse>::new(
          Box::new(std::io::Error::other(format!(
            "Pipeline execution failed: {:?}",
            e
          ))),
          crate::error::ErrorContext {
            timestamp: Utc::now(),
            item: None,
            component_name: "pipeline".to_string(),
            component_type: "Pipeline".to_string(),
          },
          crate::error::ComponentInfo {
            name: "pipeline".to_string(),
            type_name: "Pipeline".to_string(),
          },
        ),
        is_development_mode(),
      );
      HttpServerResponse::json(StatusCode::INTERNAL_SERVER_ERROR, &error_response)
        .unwrap_or_else(|_| {
          HttpServerResponse::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Pipeline execution failed: {:?}", e),
          )
        })
        .to_axum_response()
    }
  }
}

/// Creates a handler function for use with HttpGraphServer.
///
/// This function creates an Axum handler that processes requests through
/// a long-lived graph server.
///
/// ## Example
///
/// ```rust,no_run
/// use crate::http_server::{HttpGraphServer, HttpGraphServerConfig};
/// use crate::http_server::create_graph_handler;
/// use crate::graph::GraphBuilder;
/// use axum::{Router, routing::get};
///
/// // Build your graph...
/// let graph = GraphBuilder::new().build();
///
/// // Create the graph server
/// let (server, request_receiver) = HttpGraphServer::new(
///     graph,
///     HttpGraphServerConfig::default()
/// ).await?;
///
/// // Start the server
/// server.start().await?;
///
/// // Create the handler
/// let handler = create_graph_handler(server);
///
/// // Use with Axum
/// let app = Router::new()
///     .route("/api/*", get(handler));
/// ```
///
/// ## Arguments
///
/// * `server` - The HttpGraphServer instance
///
/// ## Returns
///
/// An Axum handler function
pub fn create_graph_handler(
  server: HttpGraphServer,
) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
+ Send
+ Sync
+ Clone
+ 'static {
  server.create_handler()
}
