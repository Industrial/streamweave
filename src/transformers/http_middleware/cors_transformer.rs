use crate::http::{
  http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
};
use crate::input::Input;
use crate::output::Output;
use crate::transformer::Transformer;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use std::pin::Pin;

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
  pub allowed_origins: Vec<String>,
  pub allowed_methods: Vec<Method>,
  pub allowed_headers: Vec<String>,
  pub exposed_headers: Vec<String>,
  pub allow_credentials: bool,
  pub max_age: Option<u64>,
}

impl Default for CorsConfig {
  fn default() -> Self {
    Self {
      allowed_origins: vec!["*".to_string()],
      allowed_methods: vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
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

/// CORS transformer
pub struct CorsTransformer {
  config: CorsConfig,
  transformer_config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl CorsTransformer {
  pub fn new() -> Self {
    Self {
      config: CorsConfig::default(),
      transformer_config: TransformerConfig::default(),
    }
  }

  pub fn with_config(mut self, config: CorsConfig) -> Self {
    self.config = config;
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
  fn is_origin_allowed(&self, origin: &str) -> bool {
    self.config.allowed_origins.contains(&"*".to_string())
      || self.config.allowed_origins.contains(&origin.to_string())
  }

  #[allow(dead_code)]
  fn is_method_allowed(&self, method: &Method) -> bool {
    self.config.allowed_methods.contains(method)
  }

  #[allow(dead_code)]
  fn is_header_allowed(&self, header: &str) -> bool {
    let header_lower = header.to_lowercase();
    self.config.allowed_headers.contains(&"*".to_string())
      || self
        .config
        .allowed_headers
        .iter()
        .any(|h| h.to_lowercase() == header_lower)
  }

  #[allow(dead_code)]
  fn handle_preflight_request(
    &self,
    request: &StreamWeaveHttpRequestChunk,
  ) -> StreamWeaveHttpResponse {
    let origin = request
      .headers
      .get("origin")
      .and_then(|h| h.to_str().ok())
      .unwrap_or("");

    let mut response_headers = HeaderMap::new();

    // Check if origin is allowed
    if !self.is_origin_allowed(origin) {
      return StreamWeaveHttpResponse::new(
        StatusCode::FORBIDDEN,
        response_headers,
        bytes::Bytes::from("CORS: Origin not allowed"),
      );
    }

    // Add CORS headers
    if let Ok(origin_value) = HeaderValue::from_str(origin) {
      response_headers.insert("access-control-allow-origin", origin_value);
    }

    if self.config.allow_credentials {
      response_headers.insert(
        "access-control-allow-credentials",
        HeaderValue::from_static("true"),
      );
    }

    // Handle Access-Control-Request-Method
    if let Some(request_method) = request.headers.get("access-control-request-method") {
      if let Ok(method_str) = request_method.to_str() {
        if let Ok(method) = method_str.parse::<Method>() {
          if !self.is_method_allowed(&method) {
            return StreamWeaveHttpResponse::new(
              StatusCode::FORBIDDEN,
              response_headers,
              bytes::Bytes::from("CORS: Method not allowed"),
            );
          }
        }
      }
    }

    // Add allowed methods
    let methods_str = self
      .config
      .allowed_methods
      .iter()
      .map(|m| m.as_str())
      .collect::<Vec<_>>()
      .join(", ");
    if let Ok(methods_value) = HeaderValue::from_str(&methods_str) {
      response_headers.insert("access-control-allow-methods", methods_value);
    }

    // Handle Access-Control-Request-Headers
    if let Some(request_headers) = request.headers.get("access-control-request-headers") {
      if let Ok(headers_str) = request_headers.to_str() {
        let headers: Vec<&str> = headers_str.split(',').map(|h| h.trim()).collect();
        for header in headers {
          if !self.is_header_allowed(header) {
            return StreamWeaveHttpResponse::new(
              StatusCode::FORBIDDEN,
              response_headers,
              bytes::Bytes::from("CORS: Header not allowed"),
            );
          }
        }
      }
    }

    // Add allowed headers
    let headers_str = self.config.allowed_headers.join(", ");
    if let Ok(headers_value) = HeaderValue::from_str(&headers_str) {
      response_headers.insert("access-control-allow-headers", headers_value);
    }

    // Add exposed headers
    if !self.config.exposed_headers.is_empty() {
      let exposed_headers_str = self.config.exposed_headers.join(", ");
      if let Ok(exposed_headers_value) = HeaderValue::from_str(&exposed_headers_str) {
        response_headers.insert("access-control-expose-headers", exposed_headers_value);
      }
    }

    // Add max age
    if let Some(max_age) = self.config.max_age {
      if let Ok(max_age_value) = HeaderValue::from_str(&max_age.to_string()) {
        response_headers.insert("access-control-max-age", max_age_value);
      }
    }

    StreamWeaveHttpResponse::new(StatusCode::OK, response_headers, bytes::Bytes::new())
  }

  #[allow(dead_code)]
  fn add_cors_headers_to_response(&self, response: &mut StreamWeaveHttpResponse, origin: &str) {
    if !self.is_origin_allowed(origin) {
      return;
    }

    let headers = &mut response.headers;

    // Add Access-Control-Allow-Origin
    if let Ok(origin_value) = HeaderValue::from_str(origin) {
      headers.insert("access-control-allow-origin", origin_value);
    }

    if self.config.allow_credentials {
      headers.insert(
        "access-control-allow-credentials",
        HeaderValue::from_static("true"),
      );
    }

    // Add exposed headers
    if !self.config.exposed_headers.is_empty() {
      let exposed_headers_str = self.config.exposed_headers.join(", ");
      if let Ok(exposed_headers_value) = HeaderValue::from_str(&exposed_headers_str) {
        headers.insert("access-control-expose-headers", exposed_headers_value);
      }
    }
  }
}

impl Default for CorsTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for CorsTransformer {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for CorsTransformer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for CorsTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let transformer_name = self.transformer_config.name().unwrap_or("cors".to_string());
    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(request) = input_stream.next().await {
            println!("🌐 [{}] Processing request: {} {}", transformer_name, request.method, request.path());

            // Check if this is a preflight request
            if request.method == Method::OPTIONS {
                println!("   🔄 [{}] Preflight OPTIONS request detected", transformer_name);
                // For preflight requests, we need to return a response instead of continuing the pipeline
                // This is a limitation of the current transformer design - we'll yield the request
                // and let the response handling be done elsewhere
                yield request;
            } else {
                println!("   ✅ [{}] Non-preflight request, passing through", transformer_name);
                // For non-preflight requests, just pass through
                yield request;
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

/// CORS response transformer for adding CORS headers to responses
pub struct CorsResponseTransformer {
  config: CorsConfig,
}

impl CorsResponseTransformer {
  pub fn new() -> Self {
    Self {
      config: CorsConfig::default(),
    }
  }

  pub fn with_config(mut self, config: CorsConfig) -> Self {
    self.config = config;
    self
  }

  pub fn add_cors_headers(&self, response: &mut StreamWeaveHttpResponse, origin: &str) {
    if !self.config.allowed_origins.contains(&"*".to_string())
      && !self.config.allowed_origins.contains(&origin.to_string())
    {
      return;
    }

    let headers = &mut response.headers;

    // Add Access-Control-Allow-Origin
    if let Ok(origin_value) = HeaderValue::from_str(origin) {
      headers.insert("access-control-allow-origin", origin_value);
    }

    if self.config.allow_credentials {
      headers.insert(
        "access-control-allow-credentials",
        HeaderValue::from_static("true"),
      );
    }

    // Add exposed headers
    if !self.config.exposed_headers.is_empty() {
      let exposed_headers_str = self.config.exposed_headers.join(", ");
      if let Ok(exposed_headers_value) = HeaderValue::from_str(&exposed_headers_str) {
        headers.insert("access-control-expose-headers", exposed_headers_value);
      }
    }
  }
}

impl Default for CorsResponseTransformer {
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
  use http::{Method, Uri, Version};
  use uuid::Uuid;

  fn create_test_request(method: Method, headers: HeaderMap) -> StreamWeaveHttpRequestChunk {
    let connection_info = ConnectionInfo::new(
      "127.0.0.1:8080".parse().unwrap(),
      "0.0.0.0:3000".parse().unwrap(),
      Version::HTTP_11,
    );

    StreamWeaveHttpRequestChunk::new(
      method,
      Uri::from_static("/test"),
      headers,
      bytes::Bytes::new(),
      connection_info,
      true,
      Uuid::new_v4(),
      Uuid::new_v4(),
    )
  }

  #[test]
  fn test_cors_config_default() {
    let config = CorsConfig::default();
    assert!(config.allowed_origins.contains(&"*".to_string()));
    assert!(config.allowed_methods.contains(&Method::GET));
    assert!(config.allowed_methods.contains(&Method::POST));
    assert!(config.allowed_headers.contains(&"content-type".to_string()));
    assert_eq!(config.allow_credentials, false);
    assert_eq!(config.max_age, Some(86400));
  }

  #[test]
  fn test_origin_allowed() {
    let mut config = CorsConfig::default();
    config.allowed_origins = vec![
      "https://example.com".to_string(),
      "https://test.com".to_string(),
    ];

    let transformer = CorsTransformer::new().with_config(config);

    assert!(transformer.is_origin_allowed("https://example.com"));
    assert!(transformer.is_origin_allowed("https://test.com"));
    assert!(!transformer.is_origin_allowed("https://malicious.com"));
  }

  #[test]
  fn test_origin_allowed_wildcard() {
    let config = CorsConfig::default(); // Uses wildcard by default
    let transformer = CorsTransformer::new().with_config(config);

    assert!(transformer.is_origin_allowed("https://any-origin.com"));
    assert!(transformer.is_origin_allowed("http://localhost:3000"));
  }

  #[test]
  fn test_method_allowed() {
    let mut config = CorsConfig::default();
    config.allowed_methods = vec![Method::GET, Method::POST];

    let transformer = CorsTransformer::new().with_config(config);

    assert!(transformer.is_method_allowed(&Method::GET));
    assert!(transformer.is_method_allowed(&Method::POST));
    assert!(!transformer.is_method_allowed(&Method::DELETE));
  }

  #[test]
  fn test_header_allowed() {
    let mut config = CorsConfig::default();
    config.allowed_headers = vec!["content-type".to_string(), "authorization".to_string()];

    let transformer = CorsTransformer::new().with_config(config);

    assert!(transformer.is_header_allowed("content-type"));
    assert!(transformer.is_header_allowed("authorization"));
    assert!(transformer.is_header_allowed("Content-Type")); // Case insensitive
    assert!(!transformer.is_header_allowed("x-custom-header"));
  }

  #[test]
  fn test_preflight_request_success() {
    let mut headers = HeaderMap::new();
    headers.insert("origin", "https://example.com".parse().unwrap());
    headers.insert("access-control-request-method", "POST".parse().unwrap());
    headers.insert(
      "access-control-request-headers",
      "content-type,authorization".parse().unwrap(),
    );

    let request = create_test_request(Method::OPTIONS, headers);

    let mut config = CorsConfig::default();
    config.allowed_origins = vec!["https://example.com".to_string()];
    config.allowed_methods = vec![Method::POST];
    config.allowed_headers = vec!["content-type".to_string(), "authorization".to_string()];

    let transformer = CorsTransformer::new().with_config(config);
    let response = transformer.handle_preflight_request(&request);

    assert_eq!(response.status, StatusCode::OK);
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

  #[test]
  fn test_preflight_request_origin_not_allowed() {
    let mut headers = HeaderMap::new();
    headers.insert("origin", "https://malicious.com".parse().unwrap());

    let request = create_test_request(Method::OPTIONS, headers);

    let mut config = CorsConfig::default();
    config.allowed_origins = vec!["https://example.com".to_string()];

    let transformer = CorsTransformer::new().with_config(config);
    let response = transformer.handle_preflight_request(&request);

    assert_eq!(response.status, StatusCode::FORBIDDEN);
  }

  #[test]
  fn test_preflight_request_method_not_allowed() {
    let mut headers = HeaderMap::new();
    headers.insert("origin", "https://example.com".parse().unwrap());
    headers.insert("access-control-request-method", "DELETE".parse().unwrap());

    let request = create_test_request(Method::OPTIONS, headers);

    let mut config = CorsConfig::default();
    config.allowed_origins = vec!["https://example.com".to_string()];
    config.allowed_methods = vec![Method::GET, Method::POST];

    let transformer = CorsTransformer::new().with_config(config);
    let response = transformer.handle_preflight_request(&request);

    assert_eq!(response.status, StatusCode::FORBIDDEN);
  }

  #[test]
  fn test_cors_response_transformer() {
    let mut config = CorsConfig::default();
    config.allowed_origins = vec!["https://example.com".to_string()];
    config.exposed_headers = vec!["x-custom-header".to_string()];
    config.allow_credentials = true;

    let transformer = CorsResponseTransformer::new().with_config(config);

    let mut response =
      StreamWeaveHttpResponse::new(StatusCode::OK, HeaderMap::new(), bytes::Bytes::from("test"));

    transformer.add_cors_headers(&mut response, "https://example.com");

    assert!(response.headers.contains_key("access-control-allow-origin"));
    assert!(
      response
        .headers
        .contains_key("access-control-allow-credentials")
    );
    assert!(
      response
        .headers
        .contains_key("access-control-expose-headers")
    );
  }
}
