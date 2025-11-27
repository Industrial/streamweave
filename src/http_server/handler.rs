//! # Axum Route Handler Integration
//!
//! This module provides Axum route handlers that integrate HTTP requests/responses
//! with StreamWeave pipelines, allowing users to define REST endpoints using
//! StreamWeave for processing.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::consumer::HttpResponseConsumer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::producer::{HttpRequestProducer, HttpRequestProducerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::types::{HttpRequest, HttpResponse};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::pipeline::PipelineBuilder;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::transformers::map::MapTransformer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::http::StatusCode;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::{body::Body, extract::Request, response::Response};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use futures::StreamExt;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::sync::Arc;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use tracing::{error, warn};

/// Creates a simple handler that processes requests with a transformer function.
///
/// This is a convenience function for simple cases where you just want to transform
/// the request and return a response, without explicitly building a full pipeline.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::create_simple_handler;
/// use streamweave::http_server::{HttpRequest, HttpResponse};
/// use axum::http::StatusCode;
/// use axum::{Router, routing::get};
///
/// let app = Router::new()
///     .route("/api/echo", get(create_simple_handler(|req: HttpRequest| {
///         HttpResponse::text(StatusCode::OK, &format!("Echo: {}", req.path))
///     })));
/// ```
///
/// ## Arguments
///
/// * `transform_fn` - A function that transforms an HttpRequest into an HttpResponse
///
/// ## Returns
///
/// An Axum handler function
pub fn create_simple_handler<F>(
  transform_fn: F,
) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send>>
where
  F: Fn(HttpRequest) -> HttpResponse + Send + Sync + Clone + 'static,
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
  F: Fn(HttpRequest) -> HttpResponse,
{
  // Create HTTP request producer
  let mut producer = HttpRequestProducer::from_axum_request(
    axum_request,
    HttpRequestProducerConfig::default()
      .with_extract_body(true)
      .with_parse_json(true),
  )
  .await;

  // Get the request from the producer
  let mut request_stream = producer.produce();
  if let Some(request) = request_stream.next().await {
    // Transform the request
    let response = transform_fn(request);
    response.to_axum_response()
  } else {
    // No request available
    warn!("No request available from producer");
    HttpResponse::error(StatusCode::BAD_REQUEST, "Failed to extract request").to_axum_response()
  }
}

/// Creates a handler that processes requests through a full StreamWeave pipeline.
///
/// This function builds a complete pipeline with HttpRequestProducer, a transformer,
/// and HttpResponseConsumer, then runs it. The transformer should convert HttpRequest
/// to HttpResponse.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::create_pipeline_handler;
/// use streamweave::transformers::map::MapTransformer;
/// use streamweave::http_server::{HttpRequest, HttpResponse};
/// use axum::http::StatusCode;
/// use axum::{Router, routing::get};
///
/// let app = Router::new()
///     .route("/api/process", get(create_pipeline_handler(|| {
///         MapTransformer::new(|req: HttpRequest| {
///             HttpResponse::text(StatusCode::OK, "Processed")
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
  T: crate::transformer::Transformer + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  HttpRequestProducer: crate::producer::Producer,
  <HttpRequestProducer as crate::output::Output>::Output:
    std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpRequestProducer as crate::output::Output>::OutputStream: 'static,
  T::InputStream: From<<HttpRequestProducer as crate::output::Output>::OutputStream>,
  HttpResponseConsumer: crate::consumer::Consumer,
  <HttpResponseConsumer as crate::input::Input>::Input:
    std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpResponseConsumer as crate::input::Input>::InputStream: From<T::OutputStream>,
{
  let transformer_fn = Arc::new(transformer_fn);
  move |request: Request| {
    let transformer_fn = transformer_fn.clone();
    Box::pin(async move { handle_request_with_pipeline(request, &*transformer_fn).await })
  }
}

/// Internal function that handles a request through a pipeline.
async fn handle_request_with_pipeline<T>(
  axum_request: Request,
  transformer_fn: &dyn Fn() -> T,
) -> Response<Body>
where
  T: crate::transformer::Transformer + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  HttpRequestProducer: crate::producer::Producer,
  <HttpRequestProducer as crate::output::Output>::Output:
    std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpRequestProducer as crate::output::Output>::OutputStream: 'static,
  T::InputStream: From<<HttpRequestProducer as crate::output::Output>::OutputStream>,
  HttpResponseConsumer: crate::consumer::Consumer,
  <HttpResponseConsumer as crate::input::Input>::Input:
    std::fmt::Debug + Clone + Send + Sync + 'static,
  <HttpResponseConsumer as crate::input::Input>::InputStream: From<T::OutputStream>,
{
  // Create HTTP request producer from Axum request
  let mut producer = HttpRequestProducer::from_axum_request(
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
    .consumer(consumer);

  // Run the pipeline
  match pipeline.run().await {
    Ok((_, mut consumer)) => {
      // Get the response from the consumer
      consumer.get_response().await
    }
    Err(e) => {
      error!(error = ?e, "Pipeline execution failed");
      HttpResponse::error(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("Pipeline execution failed: {:?}", e),
      )
      .to_axum_response()
    }
  }
}
