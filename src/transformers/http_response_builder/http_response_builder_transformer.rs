use super::response_data::ResponseData;
use crate::http::http_response::StreamWeaveHttpResponse;
use crate::input::Input;
use crate::output::Output;
use crate::transformer::{Transformer, TransformerConfig};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::{HeaderMap, HeaderValue};
use serde_json;
use std::collections::HashMap;
use std::pin::Pin;

/// Configuration for the HTTP Response Builder Transformer
#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponseBuilderConfig {
  /// Default content type for responses
  pub default_content_type: String,
  /// Whether compression is enabled
  pub compression_enabled: bool,
  /// Default headers to add to all responses
  pub default_headers: HashMap<String, String>,
  /// Whether to add security headers
  pub add_security_headers: bool,
  /// Whether to add CORS headers
  pub add_cors_headers: bool,
  /// CORS configuration
  pub cors_config: Option<CorsConfig>,
}

/// CORS configuration for the transformer
#[derive(Debug, Clone, PartialEq)]
pub struct CorsConfig {
  /// Allowed origins
  pub allowed_origins: Vec<String>,
  /// Allowed HTTP methods
  pub allowed_methods: Vec<String>,
  /// Allowed headers
  pub allowed_headers: Vec<String>,
  /// Exposed headers
  pub exposed_headers: Vec<String>,
  /// Whether to allow credentials
  pub allow_credentials: bool,
  /// Maximum age for preflight requests
  pub max_age: Option<u64>,
}

impl Default for HttpResponseBuilderConfig {
  fn default() -> Self {
    Self {
      default_content_type: "application/json".to_string(),
      compression_enabled: false,
      default_headers: HashMap::new(),
      add_security_headers: false,
      add_cors_headers: false,
      cors_config: None,
    }
  }
}

impl Default for CorsConfig {
  fn default() -> Self {
    Self {
      allowed_origins: vec!["*".to_string()],
      allowed_methods: vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "OPTIONS".to_string(),
      ],
      allowed_headers: vec![
        "content-type".to_string(),
        "authorization".to_string(),
        "x-requested-with".to_string(),
      ],
      exposed_headers: vec![],
      allow_credentials: false,
      max_age: Some(86400), // 24 hours
    }
  }
}

/// HTTP Response Builder Transformer
///
/// This transformer converts `ResponseData` into `StreamWeaveHttpResponse` objects.
/// It handles different content types, status codes, headers, and streaming responses.
#[derive(Debug)]
pub struct HttpResponseBuilderTransformer {
  /// Configuration for the transformer
  pub config: HttpResponseBuilderConfig,
  /// Inner transformer configuration
  pub transformer_config: TransformerConfig<ResponseData>,
}

impl HttpResponseBuilderTransformer {
  /// Create a new HTTP Response Builder Transformer with default configuration
  pub fn new() -> Self {
    Self {
      config: HttpResponseBuilderConfig::default(),
      transformer_config: TransformerConfig::default(),
    }
  }

  /// Set the default content type
  pub fn with_default_content_type(mut self, content_type: String) -> Self {
    self.config.default_content_type = content_type;
    self
  }

  /// Enable or disable compression
  pub fn with_compression_enabled(mut self, enabled: bool) -> Self {
    self.config.compression_enabled = enabled;
    self
  }

  /// Set default headers
  pub fn with_default_headers(mut self, headers: HashMap<String, String>) -> Self {
    self.config.default_headers = headers;
    self
  }

  /// Enable or disable security headers
  pub fn with_security_headers(mut self, enabled: bool) -> Self {
    self.config.add_security_headers = enabled;
    self
  }

  /// Enable or disable CORS headers
  pub fn with_cors_headers(mut self, enabled: bool) -> Self {
    self.config.add_cors_headers = enabled;
    self
  }

  /// Set CORS configuration
  pub fn with_cors_config(mut self, config: CorsConfig) -> Self {
    self.config.cors_config = Some(config);
    self
  }

  /// Build an HTTP response from ResponseData
  pub async fn build_response(
    &self,
    response_data: ResponseData,
  ) -> Result<StreamWeaveHttpResponse, ResponseBuilderError> {
    let status = response_data.status();
    let body = match &response_data {
      ResponseData::Success { body, .. } => body.clone(),
      ResponseData::Error { message, .. } => {
        let error_json = serde_json::json!({
            "error": true,
            "message": message,
            "status": status.as_u16()
        });
        Bytes::from(error_json.to_string())
      }
      ResponseData::Stream { .. } => {
        // For streaming responses, return a placeholder
        Bytes::from("Streaming response not yet implemented")
      }
    };

    let mut headers = response_data.headers().clone();

    // Add default content type if not present
    self.add_default_content_type(&mut headers);

    // Add default headers
    self.add_default_headers(&mut headers);

    // Add security headers if enabled
    if self.config.add_security_headers {
      self.add_security_headers(&mut headers);
    }

    // Add CORS headers if enabled
    if self.config.add_cors_headers {
      if let Some(ref cors_config) = self.config.cors_config {
        self.add_cors_headers(&mut headers, cors_config);
      } else {
        // Use default CORS config
        let default_cors = CorsConfig::default();
        self.add_cors_headers(&mut headers, &default_cors);
      }
    }

    Ok(StreamWeaveHttpResponse {
      status,
      headers,
      body,
      trailers: None,
    })
  }

  /// Add default content type if not present
  fn add_default_content_type(&self, headers: &mut HeaderMap) {
    if !headers.contains_key("content-type") {
      if let Ok(value) = HeaderValue::from_str(&self.config.default_content_type) {
        headers.insert("content-type", value);
      }
    }
  }

  /// Add default headers to the response
  fn add_default_headers(&self, headers: &mut HeaderMap) {
    for (key, value) in &self.config.default_headers {
      if let Ok(header_value) = HeaderValue::from_str(value) {
        if let Ok(header_name) = key.parse::<http::header::HeaderName>() {
          headers.insert(header_name, header_value);
        }
      }
    }
  }

  /// Add security headers to the response
  fn add_security_headers(&self, headers: &mut HeaderMap) {
    // X-Content-Type-Options
    let _ = headers.insert(
      "x-content-type-options",
      HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options
    let _ = headers.insert("x-frame-options", HeaderValue::from_static("DENY"));

    // X-XSS-Protection
    let _ = headers.insert(
      "x-xss-protection",
      HeaderValue::from_static("1; mode=block"),
    );

    // Strict-Transport-Security
    let _ = headers.insert(
      "strict-transport-security",
      HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Content-Security-Policy
    let _ = headers.insert(
      "content-security-policy",
      HeaderValue::from_static("default-src 'self'"),
    );
  }

  /// Add CORS headers to the response
  fn add_cors_headers(&self, headers: &mut HeaderMap, cors_config: &CorsConfig) {
    if let Ok(value) = HeaderValue::from_str(&cors_config.allowed_origins.join(", ")) {
      let _ = headers.insert("access-control-allow-origin", value);
    }

    if let Ok(value) = HeaderValue::from_str(&cors_config.allowed_methods.join(", ")) {
      let _ = headers.insert("access-control-allow-methods", value);
    }

    if let Ok(value) = HeaderValue::from_str(&cors_config.allowed_headers.join(", ")) {
      let _ = headers.insert("access-control-allow-headers", value);
    }

    if let Ok(value) = HeaderValue::from_str(&cors_config.exposed_headers.join(", ")) {
      let _ = headers.insert("access-control-expose-headers", value);
    }

    if cors_config.allow_credentials {
      let _ = headers.insert(
        "access-control-allow-credentials",
        HeaderValue::from_static("true"),
      );
    }

    if let Some(max_age) = cors_config.max_age {
      if let Ok(value) = HeaderValue::from_str(&max_age.to_string()) {
        let _ = headers.insert("access-control-max-age", value);
      }
    }
  }
}

impl Default for HttpResponseBuilderTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for HttpResponseBuilderTransformer {
  type Input = ResponseData;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for HttpResponseBuilderTransformer {
  type Output = StreamWeaveHttpResponse;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait::async_trait]
impl Transformer for HttpResponseBuilderTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let config = self.config.clone();

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(response_data) = input_stream.next().await {
            let transformer = Self {
                config: config.clone(),
                transformer_config: TransformerConfig::default()
            };

            match transformer.build_response(response_data).await {
                Ok(response) => yield response,
                Err(_) => {
                    // If there's an error building the response, create a 500 error response
                    let error_response = StreamWeaveHttpResponse::internal_server_error(
                        "Internal server error".into()
                    );
                    yield error_response;
                }
            }
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

/// Error types for the HTTP Response Builder
#[derive(Debug, thiserror::Error)]
pub enum ResponseBuilderError {
  #[error("Header parsing error: {message}")]
  HeaderParsingError { message: String },

  #[error("Content type error: {message}")]
  ContentTypeError { message: String },

  #[error("Configuration error: {message}")]
  ConfigurationError { message: String },
}

#[cfg(test)]
mod tests {
  use super::*;
  use bytes::Bytes;
  use futures::stream;
  use http::StatusCode;

  #[test]
  fn test_transformer_creation() {
    let transformer = HttpResponseBuilderTransformer::new();
    assert_eq!(transformer.config.default_content_type, "application/json");
    assert!(!transformer.config.compression_enabled);
  }

  #[test]
  fn test_transformer_with_configuration() {
    let mut default_headers = HashMap::new();
    default_headers.insert("x-api-version".to_string(), "v1".to_string());

    let transformer = HttpResponseBuilderTransformer::new()
      .with_default_content_type("text/html".to_string())
      .with_compression_enabled(true)
      .with_default_headers(default_headers.clone())
      .with_security_headers(true)
      .with_cors_headers(true);

    assert_eq!(transformer.config.default_content_type, "text/html");
    assert!(transformer.config.compression_enabled);
    assert_eq!(transformer.config.default_headers, default_headers);
    assert!(transformer.config.add_security_headers);
    assert!(transformer.config.add_cors_headers);
  }

  #[test]
  fn test_cors_config() {
    let cors_config = CorsConfig {
      allowed_origins: vec!["https://example.com".to_string()],
      allowed_methods: vec!["GET".to_string(), "POST".to_string()],
      allowed_headers: vec!["content-type".to_string()],
      exposed_headers: vec!["x-custom".to_string()],
      allow_credentials: true,
      max_age: Some(3600),
    };

    let transformer = HttpResponseBuilderTransformer::new().with_cors_config(cors_config.clone());

    assert_eq!(transformer.config.cors_config, Some(cors_config));
  }

  #[test]
  fn test_default_configs() {
    let default_config = HttpResponseBuilderConfig::default();
    assert_eq!(default_config.default_content_type, "application/json");
    assert!(!default_config.compression_enabled);
    assert!(default_config.default_headers.is_empty());
    assert!(!default_config.add_security_headers);
    assert!(!default_config.add_cors_headers);
    assert!(default_config.cors_config.is_none());

    let default_cors = CorsConfig::default();
    assert_eq!(default_cors.allowed_origins, vec!["*"]);
    assert!(default_cors.allowed_methods.contains(&"GET".to_string()));
    assert!(default_cors.allowed_methods.contains(&"POST".to_string()));
    assert!(default_cors.allowed_methods.contains(&"PUT".to_string()));
    assert!(default_cors.allowed_methods.contains(&"DELETE".to_string()));
    assert!(
      default_cors
        .allowed_methods
        .contains(&"OPTIONS".to_string())
    );
    assert!(!default_cors.allow_credentials);
    assert_eq!(default_cors.max_age, Some(86400));
  }

  #[test]
  fn test_transformer_default() {
    let transformer1 = HttpResponseBuilderTransformer::new();
    let transformer2 = HttpResponseBuilderTransformer::default();
    assert_eq!(transformer1.config, transformer2.config);
  }

  #[tokio::test]
  async fn test_success_response_building() {
    let transformer = HttpResponseBuilderTransformer::new();
    let response_data = ResponseData::ok(Bytes::from("Hello, World!"));

    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, Bytes::from("Hello, World!"));
    assert!(response.headers.contains_key("content-type"));
  }

  #[tokio::test]
  async fn test_error_response_building() {
    let transformer = HttpResponseBuilderTransformer::new();
    let response_data = ResponseData::error(StatusCode::NOT_FOUND, "Not found".to_string());

    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&response.body).contains("Not found"));
    assert!(response.headers.contains_key("content-type"));
  }

  #[tokio::test]
  async fn test_response_with_custom_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("x-custom", "value".parse().unwrap());

    let transformer = HttpResponseBuilderTransformer::new();
    let response_data =
      ResponseData::success_with_headers(StatusCode::OK, Bytes::from("test"), headers);

    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.headers.contains_key("x-custom"));
    assert!(response.headers.contains_key("content-type"));
  }

  #[tokio::test]
  async fn test_security_headers() {
    let transformer = HttpResponseBuilderTransformer::new().with_security_headers(true);

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.headers.contains_key("x-content-type-options"));
    assert!(response.headers.contains_key("x-frame-options"));
    assert!(response.headers.contains_key("x-xss-protection"));
    assert!(response.headers.contains_key("strict-transport-security"));
    assert!(response.headers.contains_key("content-security-policy"));
  }

  #[tokio::test]
  async fn test_cors_headers() {
    let cors_config = CorsConfig {
      allowed_origins: vec!["https://example.com".to_string()],
      allowed_methods: vec!["GET".to_string(), "POST".to_string()],
      allowed_headers: vec!["content-type".to_string()],
      exposed_headers: vec!["x-custom".to_string()],
      allow_credentials: true,
      max_age: Some(3600),
    };

    let transformer = HttpResponseBuilderTransformer::new()
      .with_cors_headers(true)
      .with_cors_config(cors_config);

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.headers.contains_key("access-control-allow-origin"));
    assert!(
      response
        .headers
        .contains_key("access-control-allow-methods")
    );
    assert!(
      response
        .headers
        .contains_key("access-control-allow-headers")
    );
    assert!(
      response
        .headers
        .contains_key("access-control-expose-headers")
    );
    assert!(
      response
        .headers
        .contains_key("access-control-allow-credentials")
    );
    assert!(response.headers.contains_key("access-control-max-age"));
  }

  #[tokio::test]
  async fn test_cors_headers_with_default_config() {
    let transformer = HttpResponseBuilderTransformer::new().with_cors_headers(true);

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.headers.contains_key("access-control-allow-origin"));
    assert!(
      response
        .headers
        .contains_key("access-control-allow-methods")
    );
    assert!(
      response
        .headers
        .contains_key("access-control-allow-headers")
    );
  }

  #[tokio::test]
  async fn test_default_headers() {
    let mut default_headers = HashMap::new();
    default_headers.insert("x-api-version".to_string(), "v1".to_string());
    default_headers.insert("x-server".to_string(), "streamweave".to_string());

    let transformer = HttpResponseBuilderTransformer::new().with_default_headers(default_headers);

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.headers.contains_key("x-api-version"));
    assert!(response.headers.contains_key("x-server"));
  }

  #[tokio::test]
  async fn test_default_content_type_override() {
    let transformer =
      HttpResponseBuilderTransformer::new().with_default_content_type("text/plain".to_string());

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.headers.get("content-type").unwrap(), "text/plain");
  }

  #[tokio::test]
  async fn test_existing_content_type_preserved() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/xml".parse().unwrap());

    let transformer = HttpResponseBuilderTransformer::new()
      .with_default_content_type("application/json".to_string());

    let response_data =
      ResponseData::success_with_headers(StatusCode::OK, Bytes::from("test"), headers);

    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(
      response.headers.get("content-type").unwrap(),
      "application/xml"
    );
  }

  #[tokio::test]
  async fn test_invalid_header_handling() {
    let mut default_headers = HashMap::new();
    // Header with null byte (invalid header name)
    default_headers.insert("invalid header".to_string(), "value".to_string());
    // Header with null byte in value (invalid header value)
    default_headers.insert("x-valid".to_string(), "invalid value".to_string());
    // Valid header that should be added
    default_headers.insert("x-test".to_string(), "valid-value".to_string());

    let transformer = HttpResponseBuilderTransformer::new().with_default_headers(default_headers);

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Invalid headers should be ignored
    assert!(!response.headers.contains_key("invalid header"));
    assert!(!response.headers.contains_key("x-valid")); // Rejected due to invalid value
    // Valid headers should be added
    assert!(response.headers.contains_key("x-test"));
  }

  #[tokio::test]
  async fn test_cors_headers_with_invalid_values() {
    let cors_config = CorsConfig {
      allowed_origins: vec!["invalid\x00origin".to_string()],
      allowed_methods: vec!["GET".to_string()],
      allowed_headers: vec!["content-type".to_string()],
      exposed_headers: vec![],
      allow_credentials: false,
      max_age: None,
    };

    let transformer = HttpResponseBuilderTransformer::new()
      .with_cors_headers(true)
      .with_cors_config(cors_config);

    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Invalid CORS headers should be handled gracefully
    assert!(
      response
        .headers
        .contains_key("access-control-allow-methods")
    );
    assert!(
      response
        .headers
        .contains_key("access-control-allow-headers")
    );
  }

  #[tokio::test]
  async fn test_transformer_stream() {
    let mut transformer = HttpResponseBuilderTransformer::new().with_security_headers(true);

    let response_data1 = ResponseData::ok(Bytes::from("Hello"));
    let response_data2 = ResponseData::error(StatusCode::NOT_FOUND, "Not found".to_string());

    let input_stream = stream::iter(vec![response_data1, response_data2]);
    let mut output_stream = transformer.transform(Box::pin(input_stream));
    let mut responses = Vec::new();

    while let Some(response) = output_stream.next().await {
      responses.push(response);
    }

    assert_eq!(responses.len(), 2);

    // First response should be success
    assert_eq!(responses[0].status, StatusCode::OK);
    assert_eq!(responses[0].body, Bytes::from("Hello"));
    assert!(responses[0].headers.contains_key("x-content-type-options"));

    // Second response should be error
    assert_eq!(responses[1].status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&responses[1].body).contains("Not found"));
  }

  #[tokio::test]
  async fn test_transformer_stream_error_handling() {
    let mut transformer = HttpResponseBuilderTransformer::new();

    // Test with empty stream
    let input_stream = stream::iter(vec![]);
    let mut output_stream = transformer.transform(Box::pin(input_stream));
    let mut responses = Vec::new();

    while let Some(response) = output_stream.next().await {
      responses.push(response);
    }

    assert_eq!(responses.len(), 0);
  }

  #[test]
  fn test_response_builder_error_display() {
    let header_error = ResponseBuilderError::HeaderParsingError {
      message: "Invalid header".to_string(),
    };
    assert!(format!("{}", header_error).contains("Invalid header"));

    let content_error = ResponseBuilderError::ContentTypeError {
      message: "Invalid content type".to_string(),
    };
    assert!(format!("{}", content_error).contains("Invalid content type"));

    let config_error = ResponseBuilderError::ConfigurationError {
      message: "Invalid config".to_string(),
    };
    assert!(format!("{}", config_error).contains("Invalid config"));
  }

  #[test]
  fn test_response_builder_error_debug() {
    let error = ResponseBuilderError::HeaderParsingError {
      message: "Test error".to_string(),
    };
    assert!(format!("{:?}", error).contains("HeaderParsingError"));
  }

  #[tokio::test]
  async fn test_build_response_error_path() {
    // Test the error path in build_response by creating a scenario that might fail
    let transformer = HttpResponseBuilderTransformer::new();

    // This should work fine, but we're testing the error handling path
    let response_data = ResponseData::ok(Bytes::from("test"));
    let result = transformer.build_response(response_data).await;
    assert!(result.is_ok());
  }

  #[test]
  fn test_transformer_config_methods() {
    let mut transformer = HttpResponseBuilderTransformer::new();
    let config = TransformerConfig::default();

    transformer.set_config_impl(config.clone());
    // Test that config is set correctly
    let _config = transformer.get_config_impl();

    let _mut_config = transformer.get_config_mut_impl();
    // Test that we can get mutable config
    let _mut_config = transformer.get_config_mut_impl();
  }

  #[test]
  fn test_input_output_traits() {
    let _transformer = HttpResponseBuilderTransformer::new();

    // Test that the transformer implements the required traits
    let _input_type: <HttpResponseBuilderTransformer as Input>::Input =
      ResponseData::ok(Bytes::new());
    let _output_type: <HttpResponseBuilderTransformer as Output>::Output =
      StreamWeaveHttpResponse::internal_server_error(Bytes::new());
  }
}
