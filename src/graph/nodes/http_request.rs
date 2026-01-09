//! HTTP request node for StreamWeave graphs
//!
//! Makes HTTP requests from stream items.

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
