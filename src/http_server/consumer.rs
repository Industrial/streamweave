//! # HTTP Response Consumer
//!
//! Consumer that converts stream items into HTTP responses for sending back to clients
//! via Axum.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::consumer::{Consumer, ConsumerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::types::HttpResponse;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use async_trait::async_trait;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::{body::Body, response::Response};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use futures::StreamExt;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use tracing::{error, warn};

/// Configuration for HTTP response consumer behavior.
#[derive(Debug, Clone)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
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

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
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

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
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
/// This consumer accepts `HttpResponse` items from a pipeline and converts them
/// into Axum `Response<Body>` that can be sent back to clients. It supports
/// both single-item and multi-item responses, as well as streaming responses.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};
/// use streamweave::http_server::HttpResponse;
/// use axum::http::StatusCode;
///
/// async fn handle_response(mut consumer: HttpResponseConsumer) {
///     let response = consumer.get_response().await;
///     // Send response to client
/// }
/// ```
#[derive(Debug)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub struct HttpResponseConsumer {
  /// Consumer configuration.
  pub config: ConsumerConfig<HttpResponse>,
  /// HTTP response-specific configuration.
  pub http_config: HttpResponseConsumerConfig,
  /// Collected responses.
  pub responses: Vec<HttpResponse>,
  /// Whether the consumer has finished consuming.
  pub finished: bool,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl HttpResponseConsumer {
  /// Creates a new HTTP response consumer with default configuration.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponseConsumer;
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
  /// use streamweave::http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};
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
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<HttpResponse>) -> Self {
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
  /// use streamweave::http_server::HttpResponseConsumer;
  ///
  /// let mut consumer = HttpResponseConsumer::new();
  /// // ... consume stream ...
  /// let axum_response = consumer.get_response().await;
  /// ```
  pub async fn get_response(&mut self) -> Response<Body> {
    if self.responses.is_empty() {
      // Return empty response with default status
      HttpResponse::new(
        self.http_config.default_status,
        Vec::new(),
        crate::http_server::types::ContentType::Text,
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

  /// Gets a streaming response for chunked transfer encoding.
  ///
  /// This creates a streaming response that sends items as they arrive.
  /// Useful for large responses that should be streamed to the client.
  ///
  /// ## Example
  ///
  /// ```rust,no_run
  /// use streamweave::http_server::HttpResponseConsumer;
  ///
  /// let mut consumer = HttpResponseConsumer::new();
  /// let stream_response = consumer.get_streaming_response();
  /// ```
  pub fn get_streaming_response(&self) -> Response<Body> {
    // For now, return a simple streaming response
    // Full streaming implementation will be in subtask 16.6
    if let Some(first) = self.responses.first() {
      first.clone().to_axum_response()
    } else {
      HttpResponse::new(
        self.http_config.default_status,
        Vec::new(),
        crate::http_server::types::ContentType::Text,
      )
      .to_axum_response()
    }
  }

  /// Merges multiple responses into a single response.
  ///
  /// This combines the bodies of all responses and uses the status code
  /// from the first response.
  fn merge_responses(&mut self) -> HttpResponse {
    if self.responses.is_empty() {
      return HttpResponse::new(
        self.http_config.default_status,
        Vec::new(),
        crate::http_server::types::ContentType::Text,
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

    HttpResponse {
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
  pub fn responses(&self) -> &[HttpResponse] {
    &self.responses
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Default for HttpResponseConsumer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
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

#[async_trait]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Consumer for HttpResponseConsumer {
  /// Consumes a stream of HTTP response items.
  ///
  /// This collects all `HttpResponse` items from the stream and stores them
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

  fn set_config_impl(&mut self, config: ConsumerConfig<HttpResponse>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<HttpResponse> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<HttpResponse> {
    &mut self.config
  }
}
