use crate::http::{
  http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
};
use crate::input::Input;
use crate::output::Output;
use crate::transformer::Transformer;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

/// Log level for HTTP requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
}

impl std::fmt::Display for LogLevel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      LogLevel::Trace => write!(f, "TRACE"),
      LogLevel::Debug => write!(f, "DEBUG"),
      LogLevel::Info => write!(f, "INFO"),
      LogLevel::Warn => write!(f, "WARN"),
      LogLevel::Error => write!(f, "ERROR"),
    }
  }
}

/// HTTP request log entry
#[derive(Debug, Clone, Serialize)]
pub struct HttpLogEntry {
  pub timestamp: chrono::DateTime<chrono::Utc>,
  pub method: String,
  pub path: String,
  pub query: Option<String>,
  pub status: Option<u16>,
  pub duration_ms: Option<u64>,
  pub user_agent: Option<String>,
  pub remote_addr: Option<String>,
  pub request_id: Option<String>,
  pub body_size: Option<usize>,
  pub response_size: Option<usize>,
  pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Logger trait for different logging backends
#[async_trait]
pub trait HttpLogger: Send + Sync {
  async fn log_request(&self, entry: &HttpLogEntry);
  async fn log_response(&self, entry: &HttpLogEntry);
}

/// Console logger implementation
#[derive(Clone)]
#[allow(dead_code)]
pub struct ConsoleLogger {
  log_level: LogLevel,
  include_body: bool,
}

impl ConsoleLogger {
  pub fn new(log_level: LogLevel, include_body: bool) -> Self {
    Self {
      log_level,
      include_body,
    }
  }
}

#[async_trait]
impl HttpLogger for ConsoleLogger {
  async fn log_request(&self, entry: &HttpLogEntry) {
    if self.log_level as u8 <= LogLevel::Info as u8 {
      println!(
        "[{}] {} {} {} - {}",
        entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
        entry.method,
        entry.path,
        entry.query.as_deref().unwrap_or(""),
        entry.remote_addr.as_deref().unwrap_or("unknown")
      );
    }
  }

  async fn log_response(&self, entry: &HttpLogEntry) {
    if self.log_level as u8 <= LogLevel::Info as u8 {
      let status = entry
        .status
        .map(|s| s.to_string())
        .unwrap_or_else(|| "pending".to_string());
      let duration = entry
        .duration_ms
        .map(|d| format!("{}ms", d))
        .unwrap_or_else(|| "pending".to_string());

      println!(
        "[{}] {} {} {} - {} - {}",
        entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
        entry.method,
        entry.path,
        status,
        duration,
        entry.remote_addr.as_deref().unwrap_or("unknown")
      );
    }
  }
}

/// JSON logger implementation
#[derive(Clone)]
#[allow(dead_code)]
pub struct JsonLogger {
  log_level: LogLevel,
  include_body: bool,
}

impl JsonLogger {
  pub fn new(log_level: LogLevel, include_body: bool) -> Self {
    Self {
      log_level,
      include_body,
    }
  }
}

#[async_trait]
impl HttpLogger for JsonLogger {
  async fn log_request(&self, entry: &HttpLogEntry) {
    if self.log_level as u8 <= LogLevel::Info as u8 {
      if let Ok(json) = serde_json::to_string(entry) {
        println!("{}", json);
      }
    }
  }

  async fn log_response(&self, entry: &HttpLogEntry) {
    if self.log_level as u8 <= LogLevel::Info as u8 {
      if let Ok(json) = serde_json::to_string(entry) {
        println!("{}", json);
      }
    }
  }
}

/// Request logging transformer
pub struct RequestLoggingTransformer {
  logger: Arc<dyn HttpLogger>,
  log_level: LogLevel,
  include_body: bool,
  transformer_config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl RequestLoggingTransformer {
  pub fn new() -> Self {
    Self {
      logger: Arc::new(ConsoleLogger::new(LogLevel::Info, false)),
      log_level: LogLevel::Info,
      include_body: false,
      transformer_config: TransformerConfig::default(),
    }
  }

  pub fn with_logger(mut self, logger: Arc<dyn HttpLogger>) -> Self {
    self.logger = logger;
    self
  }

  pub fn with_log_level(mut self, log_level: LogLevel) -> Self {
    self.log_level = log_level;
    self
  }

  pub fn with_include_body(mut self, include_body: bool) -> Self {
    self.include_body = include_body;
    self
  }

  pub fn with_transformer_config(
    mut self,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
  ) -> Self {
    self.transformer_config = config;
    self
  }

  #[allow(dead_code)]
  fn create_log_entry(&self, request: &StreamWeaveHttpRequestChunk) -> HttpLogEntry {
    let timestamp = chrono::Utc::now();
    let method = request.method.to_string();
    let path = request.uri.path().to_string();
    let query = request.uri.query().map(|q| q.to_string());
    let user_agent = request
      .headers
      .get("user-agent")
      .and_then(|h| h.to_str().ok())
      .map(|s| s.to_string());
    let remote_addr = Some(request.connection_info.remote_addr.to_string());
    let request_id = request
      .headers
      .get("x-request-id")
      .and_then(|h| h.to_str().ok())
      .map(|s| s.to_string());
    let body_size = Some(request.chunk.len());
    let headers = if self.include_body {
      Some(
        request
          .headers
          .iter()
          .filter_map(|(k, v)| {
            v.to_str()
              .ok()
              .map(|s| (k.as_str().to_string(), s.to_string()))
          })
          .collect(),
      )
    } else {
      None
    };

    HttpLogEntry {
      timestamp,
      method,
      path,
      query,
      status: None,
      duration_ms: None,
      user_agent,
      remote_addr,
      request_id,
      body_size,
      response_size: None,
      headers,
    }
  }
}

impl Default for RequestLoggingTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for RequestLoggingTransformer {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for RequestLoggingTransformer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for RequestLoggingTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let logger = self.logger.clone();
    let include_body = self.include_body;

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(request) = input_stream.next().await {
            // Create log entry for the request
            let log_entry = {
                let timestamp = chrono::Utc::now();
                let method = request.method.to_string();
                let path = request.uri.path().to_string();
                let query = request.uri.query().map(|q| q.to_string());
                let user_agent = request.headers.get("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());
                let remote_addr = Some(request.connection_info.remote_addr.to_string());
                let request_id = request.headers.get("x-request-id")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());
                let body_size = Some(request.chunk.len());
                let headers = if include_body {
                    Some(request.headers.iter()
                        .filter_map(|(k, v)| {
                            v.to_str().ok().map(|s| (k.as_str().to_string(), s.to_string()))
                        })
                        .collect())
                } else { None };

                HttpLogEntry {
                    timestamp,
                    method,
                    path,
                    query,
                    status: None,
                    duration_ms: None,
                    user_agent,
                    remote_addr,
                    request_id,
                    body_size,
                    response_size: None,
                    headers,
                }
            };

            // Log the request
            logger.log_request(&log_entry).await;

            // Yield the request to continue processing
            yield request;
        }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.transformer_config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.transformer_config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.transformer_config
  }
}

/// Response logging transformer
pub struct ResponseLoggingTransformer {
  logger: Arc<dyn HttpLogger>,
  log_level: LogLevel,
  include_body: bool,
  start_times: std::collections::HashMap<String, Instant>,
}

impl ResponseLoggingTransformer {
  pub fn new() -> Self {
    Self {
      logger: Arc::new(ConsoleLogger::new(LogLevel::Info, false)),
      log_level: LogLevel::Info,
      include_body: false,
      start_times: std::collections::HashMap::new(),
    }
  }

  pub fn with_logger(mut self, logger: Arc<dyn HttpLogger>) -> Self {
    self.logger = logger;
    self
  }

  pub fn with_log_level(mut self, log_level: LogLevel) -> Self {
    self.log_level = log_level;
    self
  }

  pub fn with_include_body(mut self, include_body: bool) -> Self {
    self.include_body = include_body;
    self
  }

  pub fn record_request_start(&mut self, request_id: &str) {
    self
      .start_times
      .insert(request_id.to_string(), Instant::now());
  }

  pub fn log_response(&self, response: &StreamWeaveHttpResponse, request_id: Option<&str>) {
    let timestamp = chrono::Utc::now();
    let status = response.status.as_u16();
    let duration_ms = request_id.and_then(|id| {
      self
        .start_times
        .get(id)
        .map(|start| start.elapsed().as_millis() as u64)
    });
    let response_size = Some(response.body.len());
    let headers = if self.include_body {
      Some(
        response
          .headers
          .iter()
          .filter_map(|(k, v)| {
            v.to_str()
              .ok()
              .map(|s| (k.as_str().to_string(), s.to_string()))
          })
          .collect(),
      )
    } else {
      None
    };

    let log_entry = HttpLogEntry {
      timestamp,
      method: "RESPONSE".to_string(), // We don't have the original method here
      path: "".to_string(),           // We don't have the original path here
      query: None,
      status: Some(status),
      duration_ms,
      user_agent: None,
      remote_addr: None,
      request_id: request_id.map(|s| s.to_string()),
      body_size: None,
      response_size,
      headers,
    };

    // Log the response asynchronously
    let logger = Arc::clone(&self.logger);
    tokio::spawn(async move {
      logger.log_response(&log_entry).await;
    });
  }
}

impl Default for ResponseLoggingTransformer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::http::{
    connection_info::ConnectionInfo, http_request_chunk::StreamWeaveHttpRequestChunk,
  };
  use http::{HeaderMap, Method, Uri, Version};
  use std::str::FromStr;

  fn create_test_request(
    method: Method,
    path: &str,
    headers: HeaderMap,
  ) -> StreamWeaveHttpRequestChunk {
    let connection_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      Version::HTTP_11,
    );

    StreamWeaveHttpRequestChunk::new(
      method,
      Uri::from_str(path).unwrap(),
      headers,
      bytes::Bytes::from("test body"),
      connection_info,
      true,
    )
  }

  #[test]
  fn test_log_level_display() {
    assert_eq!(LogLevel::Trace.to_string(), "TRACE");
    assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
    assert_eq!(LogLevel::Info.to_string(), "INFO");
    assert_eq!(LogLevel::Warn.to_string(), "WARN");
    assert_eq!(LogLevel::Error.to_string(), "ERROR");
  }

  #[test]
  fn test_http_log_entry_creation() {
    let mut headers = HeaderMap::new();
    headers.insert("user-agent", "test-agent".parse().unwrap());
    headers.insert("x-request-id", "req-123".parse().unwrap());

    let request = create_test_request(Method::GET, "/test?param=value", headers);
    let transformer = RequestLoggingTransformer::new();
    let log_entry = transformer.create_log_entry(&request);

    assert_eq!(log_entry.method, "GET");
    assert_eq!(log_entry.path, "/test");
    assert_eq!(log_entry.query, Some("param=value".to_string()));
    assert_eq!(log_entry.user_agent, Some("test-agent".to_string()));
    assert_eq!(log_entry.remote_addr, Some("127.0.0.1:8080".to_string()));
    assert_eq!(log_entry.request_id, Some("req-123".to_string()));
    assert_eq!(log_entry.body_size, Some(9)); // "test body".len()
    assert_eq!(log_entry.status, None);
    assert_eq!(log_entry.duration_ms, None);
  }

  #[test]
  fn test_console_logger_creation() {
    let logger = ConsoleLogger::new(LogLevel::Debug, true);
    // Just test that it can be created
    assert_eq!(logger.log_level, LogLevel::Debug);
  }

  #[test]
  fn test_json_logger_creation() {
    let logger = JsonLogger::new(LogLevel::Info, false);
    // Just test that it can be created
    assert_eq!(logger.log_level, LogLevel::Info);
  }

  #[test]
  fn test_request_logging_transformer_creation() {
    let transformer = RequestLoggingTransformer::new()
      .with_log_level(LogLevel::Debug)
      .with_include_body(true);

    assert_eq!(transformer.log_level, LogLevel::Debug);
    assert_eq!(transformer.include_body, true);
  }

  #[test]
  fn test_response_logging_transformer_creation() {
    let transformer = ResponseLoggingTransformer::new()
      .with_log_level(LogLevel::Warn)
      .with_include_body(false);

    assert_eq!(transformer.log_level, LogLevel::Warn);
    assert_eq!(transformer.include_body, false);
  }

  #[tokio::test]
  async fn test_request_logging_transformer_stream() {
    let mut transformer = RequestLoggingTransformer::new();

    let mut headers = HeaderMap::new();
    headers.insert("user-agent", "test-agent".parse().unwrap());

    let request = create_test_request(Method::POST, "/api/test", headers);
    let input_stream = futures::stream::once(async { request });

    let mut output_stream = transformer.transform(Box::pin(input_stream));

    // The stream should yield the request
    let result = output_stream.next().await;
    assert!(result.is_some());
  }
}
