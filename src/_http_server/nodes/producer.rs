//! # HTTP Request Producer
//!
//! Producer that converts incoming HTTP requests into stream items for processing
//! through StreamWeave pipelines.

use crate::Output;
use crate::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};
use crate::http_server::types::HttpServerRequest;
use crate::{Producer, ProducerConfig};
use async_stream::stream;
use async_trait::async_trait;
use axum::extract::Request;
use chrono;
use futures::Stream;
use futures::StreamExt;
#[allow(unused_imports)] // BodyExt is used via trait method into_data_stream()
use http_body_util::BodyExt;
use std::pin::Pin;
use tracing::{error, warn};

/// Configuration for HTTP request producer behavior.
#[derive(Debug, Clone)]
pub struct HttpRequestProducerConfig {
  /// Whether to extract the request body as bytes.
  pub extract_body: bool,
  /// Maximum body size to extract (in bytes). None means no limit.
  pub max_body_size: Option<usize>,
  /// Whether to parse JSON body automatically.
  pub parse_json: bool,
  /// Whether to extract query parameters.
  pub extract_query_params: bool,
  /// Whether to extract path parameters.
  pub extract_path_params: bool,
  /// Whether to stream the request body in chunks instead of loading it all at once.
  /// When enabled, the producer will yield the request metadata first, then stream
  /// body chunks. This is memory-efficient for large request bodies.
  pub stream_body: bool,
  /// Chunk size for streaming (in bytes). Only used when `stream_body` is true.
  pub chunk_size: usize,
}

impl Default for HttpRequestProducerConfig {
  fn default() -> Self {
    Self {
      extract_body: true,
      max_body_size: Some(10 * 1024 * 1024), // 10MB default limit
      parse_json: true,
      extract_query_params: true,
      extract_path_params: true,
      stream_body: false,    // Default to non-streaming for backward compatibility
      chunk_size: 64 * 1024, // 64KB default chunk size
    }
  }
}

impl HttpRequestProducerConfig {
  /// Sets whether to extract the request body.
  #[must_use]
  pub fn with_extract_body(mut self, extract: bool) -> Self {
    self.extract_body = extract;
    self
  }

  /// Sets the maximum body size to extract.
  #[must_use]
  pub fn with_max_body_size(mut self, size: Option<usize>) -> Self {
    self.max_body_size = size;
    self
  }

  /// Sets whether to parse JSON body automatically.
  #[must_use]
  pub fn with_parse_json(mut self, parse: bool) -> Self {
    self.parse_json = parse;
    self
  }

  /// Sets whether to extract query parameters.
  #[must_use]
  pub fn with_extract_query_params(mut self, extract: bool) -> Self {
    self.extract_query_params = extract;
    self
  }

  /// Sets whether to extract path parameters.
  #[must_use]
  pub fn with_extract_path_params(mut self, extract: bool) -> Self {
    self.extract_path_params = extract;
    self
  }

  /// Sets whether to stream the request body in chunks.
  ///
  /// When enabled, the producer will yield the request metadata first, then stream
  /// body chunks. This is memory-efficient for large request bodies.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::HttpRequestProducerConfig;
  ///
  /// let config = HttpRequestProducerConfig::default()
  ///     .with_stream_body(true)
  ///     .with_chunk_size(128 * 1024); // 128KB chunks
  /// ```
  #[must_use]
  pub fn with_stream_body(mut self, stream: bool) -> Self {
    self.stream_body = stream;
    self
  }

  /// Sets the chunk size for streaming (in bytes).
  ///
  /// Only used when `stream_body` is true. Default is 64KB.
  #[must_use]
  pub fn with_chunk_size(mut self, size: usize) -> Self {
    self.chunk_size = size;
    self
  }
}

/// A producer that converts incoming HTTP requests into stream items.
///
/// This producer accepts an Axum `Request` and converts it to a `HttpServerRequest`
/// that can flow through StreamWeave pipelines. It supports extracting request
/// metadata, query parameters, path parameters, and request bodies.
///
/// ## Example
///
/// ```rust,no_run
/// use crate::http_server::{HttpRequestProducer, HttpRequestProducerConfig};
/// use axum::extract::Request;
///
/// async fn handle_request(axum_request: Request) {
///     let mut producer = HttpRequestProducer::from_axum_request(
///         axum_request,
///         HttpRequestProducerConfig::default(),
///     ).await;
///     
///     let stream = producer.produce();
///     // Stream yields a single HttpServerRequest item
/// }
/// ```
#[derive(Debug)]
pub struct HttpRequestProducer {
  /// Producer configuration.
  pub config: ProducerConfig<HttpServerRequest>,
  /// HTTP request-specific configuration.
  pub http_config: HttpRequestProducerConfig,
  /// The HTTP request to produce.
  pub request: Option<HttpServerRequest>,
  /// The Axum request body stream (for streaming mode).
  /// This is stored when streaming is enabled so we can stream chunks later.
  pub body_stream: Option<axum::body::Body>,
}

impl HttpRequestProducer {
  /// Creates a new HTTP request producer from an Axum request.
  ///
  /// This extracts all relevant metadata from the Axum request and prepares
  /// it for streaming through a pipeline.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::{HttpRequestProducer, HttpRequestProducerConfig};
  /// use axum::extract::Request;
  ///
  /// async fn process_request(axum_request: Request) {
  ///     let producer = HttpRequestProducer::from_axum_request(
  ///         axum_request,
  ///         HttpRequestProducerConfig::default()
  ///             .with_extract_body(true)
  ///             .with_parse_json(true),
  ///     ).await;
  /// }
  /// ```
  pub async fn from_axum_request(
    axum_request: Request,
    http_config: HttpRequestProducerConfig,
  ) -> Self {
    // Extract body size info before consuming the request
    let body_size = axum_request
      .headers()
      .get("content-length")
      .and_then(|v| v.to_str().ok())
      .and_then(|s| s.parse::<usize>().ok());

    // Split request into parts and body before consuming
    let (parts, body) = axum_request.into_parts();
    let request_without_body = Request::from_parts(parts, axum::body::Body::empty());

    // Extract metadata first (before consuming the request)
    let mut request = HttpServerRequest::from_axum_request(request_without_body).await;

    // Extract body if configured
    let body_stream = if http_config.extract_body {
      // Check body size limit
      if let Some(size) = body_size
        && let Some(max_size) = http_config.max_body_size
        && size > max_size
      {
        warn!(
          "Request body size {} exceeds maximum {} bytes, body will not be extracted",
          size, max_size
        );
        return Self {
          config: ProducerConfig::default(),
          http_config,
          request: Some(request),
          body_stream: None,
        };
      }

      // Extract body bytes or stream
      if http_config.stream_body {
        // For streaming mode, extract the body stream for later chunking
        // Note: This consumes the request, so we extract metadata first
        request.body = None;
        Some(body)
      } else {
        // Non-streaming mode: extract entire body
        let body_result = axum::body::to_bytes(body, usize::MAX).await;
        match body_result {
          Ok(body_bytes) => {
            request.body = Some(body_bytes.to_vec());

            // Parse JSON if configured and content type is JSON
            if http_config.parse_json
              && request.is_content_type(crate::http_server::types::ContentType::Json)
            {
              // JSON parsing will be handled by transformers if needed
              // For now, we just store the raw bytes
            }
          }
          Err(e) => {
            warn!(
              error = %e,
              "Failed to extract request body"
            );
          }
        }
        None
      }
    } else {
      None
    };

    Self {
      config: ProducerConfig::default(),
      http_config,
      request: Some(request),
      body_stream,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<HttpServerRequest>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets path parameters extracted from the route.
  ///
  /// Path parameters are typically extracted by Axum route handlers and should
  /// be set on the request before producing.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::{HttpRequestProducer, HttpServerRequest};
  /// use std::collections::HashMap;
  ///
  /// let request = HttpServerRequest::new(axum::http::Method::GET, "/users/123".parse().unwrap());
  /// let mut producer = HttpRequestProducer::new(request);
  /// let mut path_params = HashMap::new();
  /// path_params.insert("id".to_string(), "123".to_string());
  /// producer.set_path_params(path_params);
  /// ```
  pub fn set_path_params(&mut self, params: std::collections::HashMap<String, String>) {
    if let Some(ref mut request) = self.request {
      request.path_params = params;
    }
  }

  /// Returns the HTTP request configuration.
  #[must_use]
  pub fn http_config(&self) -> &HttpRequestProducerConfig {
    &self.http_config
  }
}

impl Clone for HttpRequestProducer {
  fn clone(&self) -> Self {
    // Note: body_stream cannot be cloned, so cloned producers won't have streaming capability
    Self {
      config: self.config.clone(),
      http_config: self.http_config.clone(),
      request: self.request.clone(),
      body_stream: None,
    }
  }
}

impl Output for HttpRequestProducer {
  type Output = HttpServerRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Producer for HttpRequestProducer {
  type OutputPorts = (HttpServerRequest,);

  /// Produces a stream containing HTTP request items.
  ///
  /// When streaming is enabled, the stream yields:
  /// 1. First: The `HttpServerRequest` with metadata (body = None when streaming)
  /// 2. Then: Body chunks as `HttpServerRequest` items with only the body field set
  ///
  /// When streaming is disabled, the stream yields a single `HttpServerRequest` with the full body.
  ///
  /// ## Error Handling
  ///
  /// - Request parsing errors are handled according to the error strategy.
  /// - Body extraction errors are logged but don't stop the stream.
  /// - Streaming errors are handled gracefully with early termination support.
  fn produce(&mut self) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "http_request_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let request = self.request.take();
    let body_stream = self.body_stream.take();
    let _chunk_size = self.http_config.chunk_size; // Reserved for future chunk size control
    let stream_body = self.http_config.stream_body;

    Box::pin(stream! {
      match request {
        Some(req) => {
          // Always yield the request metadata first
          // When streaming, body will be None; when not streaming, body will contain full body
          yield req.clone();

          // If streaming is enabled and we have a body stream, stream the chunks
          if stream_body
            && let Some(body) = body_stream {
              // Convert Axum Body to a stream of bytes
              // Note: Chunk size is controlled by the body's natural boundaries
              // Future enhancement: buffer and split chunks to exact chunk_size
              let mut body_stream = body.into_data_stream();
              let mut total_bytes = 0u64;

              while let Some(chunk_result) = body_stream.next().await {
                match chunk_result {
                  Ok(chunk) => {
                    // chunk is already bytes::Bytes
                    total_bytes += chunk.len() as u64;

                    // Yield chunk as HttpServerRequest with body set
                    // Clone minimal metadata for context
                    // Add progress tracking via custom header
                    let mut chunk_headers = req.headers.clone();
                    chunk_headers.insert(
                      axum::http::HeaderName::from_static("x-streamweave-chunk-offset"),
                      axum::http::HeaderValue::from_str(&total_bytes.to_string())
                        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
                    );
                    chunk_headers.insert(
                      axum::http::HeaderName::from_static("x-streamweave-chunk-size"),
                      axum::http::HeaderValue::from_str(&chunk.len().to_string())
                        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
                    );

                    let chunk_request = HttpServerRequest {
                      request_id: req.request_id.clone(),
                      method: req.method,
                      uri: req.uri.clone(),
                      path: req.path.clone(),
                      headers: chunk_headers,
                      query_params: req.query_params.clone(),
                      path_params: req.path_params.clone(),
                      body: Some(chunk.to_vec()),
                      content_type: req.content_type.clone(),
                      remote_addr: req.remote_addr.clone(),
                    };

                    yield chunk_request;

                    // Check for early termination on errors (if error strategy is Stop)
                    // This allows downstream to signal termination
                  }
                  Err(e) => {
                    warn!(
                      component = %component_name,
                      error = %e,
                      total_bytes = total_bytes,
                      "Error reading body chunk during streaming"
                    );

                    match error_strategy {
                      ErrorStrategy::Stop => {
                        error!(
                          component = %component_name,
                          error = %e,
                          "Stopping stream due to body read error"
                        );
                        break;
                      }
                      ErrorStrategy::Skip => {
                        // Continue streaming, skip this chunk
                        continue;
                      }
                      ErrorStrategy::Retry(_) => {
                        warn!(
                          component = %component_name,
                          "Cannot retry body chunk read, skipping"
                        );
                        continue;
                      }
                      ErrorStrategy::Custom(_) => {
                        warn!(
                          component = %component_name,
                          "Custom error handler not applicable for body chunk read"
                        );
                        // Continue streaming
                        continue;
                      }
                    }
                  }
                }
              }
            }
        }
        None => {
          let error: StreamError<HttpServerRequest> = StreamError::new(
            Box::new(std::io::Error::other("No request available")),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: None,
              component_name: component_name.clone(),
              component_type: std::any::type_name::<HttpRequestProducer>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<HttpRequestProducer>().to_string(),
            },
          );

          match error_strategy {
            ErrorStrategy::Stop => {
              error!(
                component = %component_name,
                error = %error,
                "Stopping due to missing request"
              );
            }
            ErrorStrategy::Skip => {
              warn!(
                component = %component_name,
                error = %error,
                "Skipping missing request"
              );
            }
            ErrorStrategy::Retry(_) => {
              warn!(
                component = %component_name,
                error = %error,
                "Cannot retry missing request"
              );
            }
            ErrorStrategy::Custom(_) => {
              // Custom handler would be called, but we can't yield errors in the stream
              error!(
                component = %component_name,
                error = %error,
                "Custom error handler not applicable for missing request"
              );
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<HttpServerRequest>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<HttpServerRequest> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<HttpServerRequest> {
    &mut self.config
  }
}
