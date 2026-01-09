//! HTTP request transformer for StreamWeave
//!
//! Makes HTTP requests from stream items, useful for graph composition.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;

/// HTTP request configuration for the transformer.
#[derive(Debug, Clone)]
pub struct HttpRequestConfig {
  /// Base URL (can be overridden per item)
  pub base_url: Option<String>,
  /// HTTP method (GET, POST, etc.)
  pub method: String,
  /// Default headers
  pub headers: HashMap<String, String>,
  /// Request timeout in seconds
  pub timeout_secs: u64,
  /// Whether to parse response as JSON
  pub parse_json: bool,
  /// Whether to include request body
  pub include_body: bool,
}

impl Default for HttpRequestConfig {
  fn default() -> Self {
    Self {
      base_url: None,
      method: "GET".to_string(),
      headers: HashMap::new(),
      timeout_secs: 30,
      parse_json: true,
      include_body: true,
    }
  }
}

/// A transformer that makes HTTP requests from stream items.
///
/// Input can be:
/// - A URL string (for simple GET requests)
/// - A JSON object with `url`, `method`, `headers`, `body` fields
/// - An HttpRequest object (when http-server feature is enabled)
///
/// Output is the HTTP response body (as string or parsed JSON).
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::HttpRequestTransformer;
///
/// // Simple GET requests
/// let transformer = HttpRequestTransformer::new();
/// // Input: ["https://api.example.com/users", "https://api.example.com/posts"]
/// // Output: [response1, response2]
/// ```
pub struct HttpRequestTransformer {
  /// Request configuration
  config: HttpRequestConfig,
  /// Transformer configuration
  transformer_config: TransformerConfig<String>,
  /// HTTP client (shared for connection pooling)
  client: Option<reqwest::Client>,
}

impl HttpRequestTransformer {
  /// Creates a new `HttpRequestTransformer` with default configuration.
  pub fn new() -> Self {
    Self {
      config: HttpRequestConfig::default(),
      transformer_config: TransformerConfig::default(),
      client: None,
    }
  }

  /// Sets the base URL for requests.
  pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
    self.config.base_url = Some(url.into());
    self
  }

  /// Sets the HTTP method.
  pub fn with_method(mut self, method: impl Into<String>) -> Self {
    self.config.method = method.into().to_uppercase();
    self
  }

  /// Adds a header.
  pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.config.headers.insert(name.into(), value.into());
    self
  }

  /// Sets the request timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.config.timeout_secs = secs;
    self
  }

  /// Sets whether to parse response as JSON.
  pub fn with_parse_json(mut self, parse: bool) -> Self {
    self.config.parse_json = parse;
    self
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer_config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer_config.name = Some(name);
    self
  }

  /// Gets or creates the HTTP client.
  fn get_client(&mut self) -> reqwest::Client {
    if self.client.is_none() {
      let timeout = Duration::from_secs(self.config.timeout_secs);
      self.client = Some(
        reqwest::Client::builder()
          .timeout(timeout)
          .build()
          .expect("Failed to create HTTP client"),
      );
    }
    self.client.as_ref().unwrap().clone()
  }
}

impl Default for HttpRequestTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for HttpRequestTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      transformer_config: self.transformer_config.clone(),
      client: None, // Client will be created on first use
    }
  }
}

impl Input for HttpRequestTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for HttpRequestTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for HttpRequestTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let config = self.config.clone();
    let client = self.get_client();
    let parse_json = config.parse_json;

    Box::pin(input.then(move |item| {
      let config = config.clone();
      let client = client.clone();
      async move {
        // Parse input - could be URL string or JSON object
        if let Ok(json) = serde_json::from_str::<Value>(&item) {
          // JSON object with url, method, headers, body fields
          let url_str = json
            .get("url")
            .and_then(|v| v.as_str())
            .or(config.base_url.as_deref())
            .unwrap_or("");
          let method = json
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or(&config.method)
            .to_uppercase();
          let headers = json.get("headers").and_then(|v| v.as_object());
          let body = json.get("body");

          if url_str.is_empty() {
            return item; // Return original on error
          }

          // Build request
          let mut request = match method.as_str() {
            "GET" => client.get(url_str),
            "POST" => {
              let mut req = client.post(url_str);
              if config.include_body
                && let Some(body_val) = body
              {
                req = req.json(body_val);
              }
              req
            }
            "PUT" => {
              let mut req = client.put(url_str);
              if config.include_body
                && let Some(body_val) = body
              {
                req = req.json(body_val);
              }
              req
            }
            "DELETE" => client.delete(url_str),
            "PATCH" => {
              let mut req = client.patch(url_str);
              if config.include_body
                && let Some(body_val) = body
              {
                req = req.json(body_val);
              }
              req
            }
            _ => client.get(url_str), // Default to GET
          };

          // Add headers
          for (key, value) in &config.headers {
            request = request.header(key, value);
          }
          if let Some(headers_obj) = headers {
            for (key, value) in headers_obj {
              if let Some(val_str) = value.as_str() {
                request = request.header(key, val_str);
              }
            }
          }

          // Send request
          match request.send().await {
            Ok(response) => {
              let response_text = response.text().await.unwrap_or(item.clone());
              if parse_json {
                match serde_json::from_str::<Value>(&response_text) {
                  Ok(json) => serde_json::to_string(&json).unwrap_or(item),
                  Err(_) => response_text,
                }
              } else {
                response_text
              }
            }
            Err(_) => item, // Return original on error
          }
        } else {
          // Simple URL string
          let url_str = if let Some(base) = &config.base_url {
            if item.starts_with("http://") || item.starts_with("https://") {
              item.clone()
            } else {
              format!("{}{}", base.trim_end_matches('/'), item)
            }
          } else {
            item.clone()
          };

          // Build request with default method
          let mut request = match config.method.as_str() {
            "GET" => client.get(&url_str),
            "POST" => client.post(&url_str),
            "PUT" => client.put(&url_str),
            "DELETE" => client.delete(&url_str),
            "PATCH" => client.patch(&url_str),
            _ => client.get(&url_str),
          };

          // Add default headers
          for (key, value) in &config.headers {
            request = request.header(key, value);
          }

          // Send request
          match request.send().await {
            Ok(response) => {
              let response_text = response.text().await.unwrap_or(item.clone());
              if parse_json {
                match serde_json::from_str::<Value>(&response_text) {
                  Ok(json) => serde_json::to_string(&json).unwrap_or(item),
                  Err(_) => response_text,
                }
              } else {
                response_text
              }
            }
            Err(_) => item, // Return original on error
          }
        }
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer_config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.transformer_config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    &mut self.transformer_config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.transformer_config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .transformer_config
        .name
        .clone()
        .unwrap_or_else(|| "http_request_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
