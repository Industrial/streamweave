//! HTTP request node for making HTTP requests in graphs.
//!
//! This module provides [`HttpRequest`], a graph node that makes HTTP requests from
//! stream items. It wraps [`HttpRequestTransformer`] for use in StreamWeave graphs.
//! It supports configurable URLs, methods, headers, timeouts, and optional JSON parsing.
//!
//! # Overview
//!
//! [`HttpRequest`] is useful for making HTTP requests in graph-based pipelines.
//! It takes stream items as request data and makes HTTP requests, optionally parsing
//! responses as JSON. This makes it ideal for API integration and HTTP-based
//! data processing workflows.
//!
//! # Key Concepts
//!
//! - **HTTP Requests**: Makes HTTP requests from stream items
//! - **Configurable Options**: Supports custom URLs, methods, headers, and timeouts
//! - **JSON Parsing**: Optionally parses responses as JSON
//! - **Transformer Wrapper**: Wraps `HttpRequestTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`HttpRequest`]**: Node that makes HTTP requests from stream items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::HttpRequest;
//!
//! // Create an HTTP request node
//! let http_request = HttpRequest::new();
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::graph::nodes::HttpRequest;
//!
//! // Create an HTTP request node with configuration
//! let http_request = HttpRequest::new()
//!     .with_base_url("https://api.example.com")
//!     .with_method("POST")
//!     .with_header("Authorization", "Bearer token")
//!     .with_timeout_secs(30)
//!     .with_parse_json(true);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::HttpRequest;
//! use streamweave::ErrorStrategy;
//!
//! // Create an HTTP request node with error handling
//! let http_request = HttpRequest::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("api-client".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **HTTP Client Integration**: Uses reqwest or similar HTTP client for
//!   async HTTP requests
//! - **Configurable Options**: Supports extensive configuration for flexible
//!   HTTP request handling
//! - **JSON Support**: Optionally parses responses as JSON for convenience
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`HttpRequest`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::HttpRequestTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that makes HTTP requests from stream items.
///
/// This node wraps `HttpRequestTransformer` for use in graphs.
pub struct HttpRequest {
  /// The underlying request transformer
  transformer: HttpRequestTransformer,
}

impl HttpRequest {
  /// Creates a new `HttpRequest` node.
  pub fn new() -> Self {
    Self {
      transformer: HttpRequestTransformer::new(),
    }
  }

  /// Sets the base URL for requests.
  pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
    self.transformer = self.transformer.with_base_url(url);
    self
  }

  /// Sets the HTTP method.
  pub fn with_method(mut self, method: impl Into<String>) -> Self {
    self.transformer = self.transformer.with_method(method);
    self
  }

  /// Adds a header.
  pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.transformer = self.transformer.with_header(name, value);
    self
  }

  /// Sets the request timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.transformer = self.transformer.with_timeout_secs(secs);
    self
  }

  /// Sets whether to parse response as JSON.
  pub fn with_parse_json(mut self, parse: bool) -> Self {
    self.transformer = self.transformer.with_parse_json(parse);
    self
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Default for HttpRequest {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for HttpRequest {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for HttpRequest {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for HttpRequest {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for HttpRequest {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
