//! # HTTP Response Consumer
//!
//! Consumer that converts stream items into HTTP responses for sending back to clients
//! via Axum.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig};
use crate::{HttpServerResponse, Input};
use async_stream::stream;
use async_trait::async_trait;
use axum::body::Bytes;
use axum::{body::Body, response::Response};
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use tracing::{error, warn};

/// Configuration for HTTP response consumer behavior.
#[derive(Debug, Clone)]
pub struct HttpResponseConsumerConfig {
  /// Whether to stream responses (chunked transfer encoding).
  pub stream_response: bool,
  /// Maximum number of items to collect before sending response.
  /// None means collect all items.
  pub max_items: Option<usize>,
  /// Whether to merge multiple responses into a single response.
  pub merge_responses: bool,
  /// Default status code if none is provided.
  pub default_status: axum::http::StatusCode,
}

impl Default for HttpResponseConsumerConfig {
  fn default() -> Self {
    Self {
      stream_response: false,
      max_items: None,
      merge_responses: false,
      default_status: axum::http::StatusCode::OK,
    }
  }
}

impl HttpResponseConsumerConfig {
  /// Sets whether to stream responses.
  #[must_use]
  pub fn with_stream_response(mut self, stream: bool) -> Self {
    self.stream_response = stream;
    self
  }

  /// Sets the maximum number of items to collect.
  #[must_use]
  pub fn with_max_items(mut self, max: Option<usize>) -> Self {
    self.max_items = max;
    self
  }

  /// Sets whether to merge multiple responses.
  #[must_use]
  pub fn with_merge_responses(mut self, merge: bool) -> Self {
    self.merge_responses = merge;
    self
  }

  /// Sets the default status code.
  #[must_use]
  pub fn with_default_status(mut self, status: axum::http::StatusCode) -> Self {
    self.default_status = status;
    self
  }
}

/// A consumer that converts stream items into HTTP responses.
///
/// This consumer accepts `HttpServerResponse` items from a pipeline and converts them
/// into Axum `Response<Body>` that can be sent back to clients. It supports
/// both single-item and multi-item responses, as well as streaming responses.
///
/// ## Example
///
/// ```rust,no_run
/// use crate::http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};
/// use crate::http_server::HttpServerResponse;
/// use axum::http::StatusCode;
///
/// async fn handle_response(mut consumer: HttpResponseConsumer) {
///     let response = consumer.get_response().await;
///     // Send response to client
/// }
/// ```
#[derive(Debug)]
pub struct HttpResponseConsumer {
  /// Consumer configuration.
  pub config: ConsumerConfig<HttpServerResponse>,
  /// HTTP response-specific configuration.
  pub http_config: HttpResponseConsumerConfig,
  /// Collected responses.
  pub responses: Vec<HttpServerResponse>,
  /// Whether the consumer has finished consuming.
  pub finished: bool,
}

impl HttpResponseConsumer {
  /// Creates a new HTTP response consumer with default configuration.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::HttpResponseConsumer;
  ///
  /// let consumer = HttpResponseConsumer::new();
  /// ```
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
      http_config: HttpResponseConsumerConfig::default(),
      responses: Vec::new(),
      finished: false,
    }
  }

  /// Creates a new HTTP response consumer with custom configuration.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};
  ///
  /// let consumer = HttpResponseConsumer::with_config(
  ///     HttpResponseConsumerConfig::default()
  ///         .with_stream_response(true)
  ///         .with_merge_responses(false),
  /// );
  /// ```
  #[must_use]
  pub fn with_config(http_config: HttpResponseConsumerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      http_config,
      responses: Vec::new(),
      finished: false,
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<HttpServerResponse>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Gets the collected response as an Axum Response.
  ///
  /// This should be called after `consume` has completed. For single-item
  /// responses, it returns the first response. For multi-item responses,
  /// it merges them according to the configuration.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::HttpResponseConsumer;
  ///
  /// let mut consumer = HttpResponseConsumer::new();
  /// // ... consume stream ...
  /// let axum_response = consumer.get_response().await;
  /// ```
  pub async fn get_response(&mut self) -> Response<Body> {
    if self.responses.is_empty() {
      // Return empty response with default status
      HttpServerResponse::new(
        self.http_config.default_status,
        Vec::new(),
        crate::graph::http_server::types::ContentType::Text,
      )
      .to_axum_response()
    } else if self.responses.len() == 1 {
      // Single response - return as-is
      self.responses.remove(0).to_axum_response()
    } else if self.http_config.merge_responses {
      // Merge multiple responses
      self.merge_responses().to_axum_response()
    } else {
      // Multiple responses - return first one (or could return array)
      // For now, return first response
      self.responses.remove(0).to_axum_response()
    }
  }

  /// Creates a streaming response from a stream of HttpServerResponse items.
  ///
  /// This method converts a stream of `HttpServerResponse` items into a streaming Axum
  /// response with chunked transfer encoding. The first response's status and headers
  /// are used, and subsequent responses' bodies are streamed as chunks.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use crate::http_server::{HttpResponseConsumer, HttpServerResponse};
  /// use crate::consumer::Consumer;
  /// use futures::StreamExt;
  /// use axum::http::StatusCode;
  ///
  /// async fn stream_responses(mut stream: impl futures::Stream<Item = HttpServerResponse> + Send) {
  ///     let consumer = HttpResponseConsumer::new();
  ///     let response = consumer.create_streaming_response(stream).await;
  ///     // Send response to client
  /// }
  /// ```
  pub async fn create_streaming_response(
    &self,
    stream: impl futures::Stream<Item = HttpServerResponse> + Send + 'static,
  ) -> Response<Body> {
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    // Clone config data needed for the stream (to avoid lifetime issues)
    let component_name = self.config.name.clone();
    let error_strategy = self.config.error_strategy.clone();
    let default_status = self.http_config.default_status;

    // Shared state for first response metadata
    let first_response_meta: Arc<Mutex<Option<(axum::http::StatusCode, axum::http::HeaderMap)>>> =
      Arc::new(Mutex::new(None));

    let first_response_meta_clone = first_response_meta.clone();

    // Pin the stream to make it usable in the async stream
    let pinned_stream = Box::pin(stream);

    let body_stream: Pin<Box<dyn futures::Stream<Item = Result<Bytes, std::io::Error>> + Send>> =
      Box::pin(stream! {
        let mut stream = pinned_stream;
        let mut first_response_handled = false;

        while let Some(response) = stream.next().await {
          // Handle first response specially - extract status and headers
          if !first_response_handled {
            let content_type = response.content_type.clone();
            let mut headers = response.headers.clone();

            // Set Content-Type header if not already set
            if !headers.contains_key("content-type") {
              let content_type_value = axum::http::HeaderValue::from_str(content_type.as_str())
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream"));
              headers.insert("content-type", content_type_value);
            }

            // Set Transfer-Encoding: chunked for streaming
            headers.insert(
              "transfer-encoding",
              axum::http::HeaderValue::from_static("chunked"),
            );

            // Store metadata in shared state
            *first_response_meta_clone.lock().unwrap() = Some((response.status, headers));
            first_response_handled = true;
          }

          // Handle errors in response stream
          if response.status.as_u16() >= 400 {
            let error = StreamError::new(
              Box::new(std::io::Error::other(format!(
                "HTTP error status: {}",
                response.status
              ))),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(response.clone()),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<HttpResponseConsumer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<HttpResponseConsumer>().to_string(),
              },
            );

            match error_strategy {
              ErrorStrategy::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping stream due to HTTP error status"
                );
                break;
              }
              ErrorStrategy::Skip => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Skipping response with error status"
                );
                continue;
              }
              ErrorStrategy::Retry(_) => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Cannot retry HTTP response streaming"
                );
                continue;
              }
              ErrorStrategy::Custom(_) => {
                warn!(
                  component = %component_name,
                  "Custom error handler not applicable for streaming"
                );
                continue;
              }
            }
          }

          // Yield response body as chunk
          if !response.body.is_empty() {
            yield Ok(Bytes::from(response.body));
          }
        }

        // If no responses were received, we need to handle that
        // But we can't return early from a stream, so we'll handle it in the response builder
      });

    // Convert stream to Axum Body
    let body = Body::from_stream(body_stream);

    // Get metadata from first response (if any)
    let (status, headers) = if let Some(meta) = first_response_meta.lock().unwrap().take() {
      meta
    } else {
      // No responses - use defaults
      let mut default_headers = axum::http::HeaderMap::new();
      default_headers.insert(
        "content-type",
        axum::http::HeaderValue::from_static("text/plain"),
      );
      (default_status, default_headers)
    };

    // Build response with status and headers
    let mut response = Response::builder().status(status).body(body).unwrap();

    // Set headers
    *response.headers_mut() = headers;

    response
  }

  /// Merges multiple responses into a single response.
  ///
  /// This combines the bodies of all responses and uses the status code
  /// from the first response.
  fn merge_responses(&mut self) -> HttpServerResponse {
    if self.responses.is_empty() {
      return HttpServerResponse::new(
        self.http_config.default_status,
        Vec::new(),
        crate::graph::http_server::types::ContentType::Text,
      );
    }

    let first = self.responses.remove(0);
    let mut merged_body = first.body.clone();
    let status = first.status;
    let content_type = first.content_type;
    let mut headers = first.headers;

    // Merge remaining responses
    for response in &self.responses {
      merged_body.extend_from_slice(&response.body);
      // Merge headers (first response takes precedence)
      for (key, value) in response.headers.iter() {
        if !headers.contains_key(key) {
          headers.insert(key.clone(), value.clone());
        }
      }
    }

    HttpServerResponse {
      request_id: first.request_id.clone(),
      status,
      headers,
      body: merged_body,
      content_type,
    }
  }

  /// Returns the HTTP response configuration.
  #[must_use]
  pub fn http_config(&self) -> &HttpResponseConsumerConfig {
    &self.http_config
  }

  /// Returns the collected responses.
  #[must_use]
  pub fn responses(&self) -> &[HttpServerResponse] {
    &self.responses
  }
}

impl Default for HttpResponseConsumer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for HttpResponseConsumer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      http_config: self.http_config.clone(),
      responses: self.responses.clone(),
      finished: self.finished,
    }
  }
}

impl Input for HttpResponseConsumer {
  type Input = HttpServerResponse;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl Consumer for HttpResponseConsumer {
  type InputPorts = (HttpServerResponse,);

  /// Consumes a stream of HTTP response items.
  ///
  /// This collects all `HttpServerResponse` items from the stream and stores them
  /// for later conversion to an Axum response via `get_response()`.
  ///
  /// ## Error Handling
  ///
  /// - Errors are handled according to the error strategy.
  /// - Invalid responses are logged but may be skipped based on strategy.
  async fn consume(&mut self, mut stream: Self::InputStream) {
    let component_name = self.config.name.clone();
    let max_items = self.http_config.max_items;
    let mut count = 0;

    while let Some(response) = stream.next().await {
      count += 1;

      // Check max items limit
      if let Some(max) = max_items
        && count > max
      {
        warn!(
          component = %component_name,
          "Maximum items limit ({}) reached, stopping consumption",
          max
        );
        break;
      }

      // Validate response
      if response.status.as_u16() >= 400 {
        let error = StreamError::new(
          Box::new(std::io::Error::other(format!(
            "HTTP error status: {}",
            response.status
          ))),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: Some(response.clone()),
            component_name: component_name.clone(),
            component_type: std::any::type_name::<HttpResponseConsumer>().to_string(),
          },
          ComponentInfo {
            name: component_name.clone(),
            type_name: std::any::type_name::<HttpResponseConsumer>().to_string(),
          },
        );

        match self.handle_error(&error) {
          ErrorAction::Stop => {
            error!(
              component = %component_name,
              error = %error,
              "Stopping due to HTTP error status"
            );
            break;
          }
          ErrorAction::Skip => {
            warn!(
              component = %component_name,
              error = %error,
              "Skipping response with error status"
            );
            continue;
          }
          ErrorAction::Retry => {
            warn!(
              component = %component_name,
              error = %error,
              "Cannot retry HTTP response consumption"
            );
            continue;
          }
        }
      }

      self.responses.push(response);
    }

    self.finished = true;
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<HttpServerResponse>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<HttpServerResponse> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<HttpServerResponse> {
    &mut self.config
  }
}

// HttpResponseCorrelationConsumer has been removed - use HttpServerConsumerNode instead
// The old implementation has been completely removed
