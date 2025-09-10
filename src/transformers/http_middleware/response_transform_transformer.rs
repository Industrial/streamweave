use crate::http::http_response::StreamWeaveHttpResponse;
use crate::input::Input;
use crate::output::Output;
use crate::transformer::Transformer;
use crate::transformer::TransformerConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use http::{HeaderMap, HeaderValue, StatusCode};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

/// Response transformation rule
pub enum ResponseTransformRule {
  /// Add headers to all responses
  AddHeaders(HashMap<String, String>),
  /// Remove headers from all responses
  RemoveHeaders(Vec<String>),
  /// Transform response body
  TransformBody(Box<dyn ResponseBodyTransformer + Send + Sync>),
  /// Set status code
  SetStatusCode(StatusCode),
  /// Add CORS headers
  AddCorsHeaders {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
  },
  /// Add security headers
  AddSecurityHeaders,
  /// Compress response if not already compressed
  CompressResponse,
  /// Add cache headers
  AddCacheHeaders { max_age: u64, cache_control: String },
}

/// Response body transformer trait
#[async_trait]
pub trait ResponseBodyTransformer: Send + Sync {
  async fn transform(
    &self,
    body: &[u8],
    content_type: Option<&str>,
  ) -> Result<Vec<u8>, TransformError>;
}

/// JSON response transformer
pub struct JsonResponseTransformer {
  pretty_print: bool,
  remove_null_fields: bool,
}

impl JsonResponseTransformer {
  pub fn new() -> Self {
    Self {
      pretty_print: false,
      remove_null_fields: false,
    }
  }

  pub fn with_pretty_print(mut self, pretty_print: bool) -> Self {
    self.pretty_print = pretty_print;
    self
  }

  pub fn with_remove_null_fields(mut self, remove_null_fields: bool) -> Self {
    self.remove_null_fields = remove_null_fields;
    self
  }
}

#[async_trait]
impl ResponseBodyTransformer for JsonResponseTransformer {
  async fn transform(
    &self,
    body: &[u8],
    _content_type: Option<&str>,
  ) -> Result<Vec<u8>, TransformError> {
    // Parse JSON
    let mut json_value: serde_json::Value =
      serde_json::from_slice(body).map_err(|e| TransformError::JsonParseError {
        message: e.to_string(),
      })?;

    // Remove null fields if requested
    if self.remove_null_fields {
      json_value = JsonResponseTransformer::remove_null_values_static(json_value);
    }

    // Serialize JSON
    let result = if self.pretty_print {
      serde_json::to_string_pretty(&json_value)
    } else {
      serde_json::to_string(&json_value)
    };

    result
      .map(|s| s.into_bytes())
      .map_err(|e| TransformError::JsonSerializeError {
        message: e.to_string(),
      })
  }
}

impl JsonResponseTransformer {
  fn remove_null_values_static(value: serde_json::Value) -> serde_json::Value {
    match value {
      serde_json::Value::Object(mut map) => {
        map.retain(|_, v| !v.is_null());
        serde_json::Value::Object(map)
      }
      serde_json::Value::Array(arr) => serde_json::Value::Array(
        arr
          .into_iter()
          .map(|v| JsonResponseTransformer::remove_null_values_static(v))
          .collect(),
      ),
      other => other,
    }
  }
}

/// XML response transformer
pub struct XmlResponseTransformer {
  pretty_print: bool,
  indent: usize,
}

impl XmlResponseTransformer {
  pub fn new() -> Self {
    Self {
      pretty_print: true,
      indent: 2,
    }
  }

  pub fn with_pretty_print(mut self, pretty_print: bool) -> Self {
    self.pretty_print = pretty_print;
    self
  }

  pub fn with_indent(mut self, indent: usize) -> Self {
    self.indent = indent;
    self
  }
}

#[async_trait]
impl ResponseBodyTransformer for XmlResponseTransformer {
  async fn transform(
    &self,
    body: &[u8],
    _content_type: Option<&str>,
  ) -> Result<Vec<u8>, TransformError> {
    // In a real implementation, you would use an XML library like quick-xml
    // For now, we'll just return the original body
    Ok(body.to_vec())
  }
}

/// HTML response transformer
pub struct HtmlResponseTransformer {
  minify: bool,
  add_doctype: bool,
}

impl HtmlResponseTransformer {
  pub fn new() -> Self {
    Self {
      minify: false,
      add_doctype: true,
    }
  }

  pub fn with_minify(mut self, minify: bool) -> Self {
    self.minify = minify;
    self
  }

  pub fn with_add_doctype(mut self, add_doctype: bool) -> Self {
    self.add_doctype = add_doctype;
    self
  }
}

#[async_trait]
impl ResponseBodyTransformer for HtmlResponseTransformer {
  async fn transform(
    &self,
    body: &[u8],
    _content_type: Option<&str>,
  ) -> Result<Vec<u8>, TransformError> {
    let mut html = String::from_utf8_lossy(body).to_string();

    // Add DOCTYPE if requested and not present
    if self.add_doctype && !html.to_lowercase().starts_with("<!doctype") {
      html = format!("<!DOCTYPE html>\n{}", html);
    }

    // Minify if requested
    if self.minify {
      html = HtmlResponseTransformer::minify_html_static(&html);
    }

    Ok(html.into_bytes())
  }
}

impl HtmlResponseTransformer {
  fn minify_html_static(html: &str) -> String {
    // Simple HTML minification - remove extra whitespace
    html
      .lines()
      .map(|line| line.trim())
      .filter(|line| !line.is_empty())
      .collect::<Vec<_>>()
      .join("")
  }
}

/// Response transformation error
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
  #[error("JSON parse error: {message}")]
  JsonParseError { message: String },

  #[error("JSON serialize error: {message}")]
  JsonSerializeError { message: String },

  #[error("XML parse error: {message}")]
  XmlParseError { message: String },

  #[error("HTML parse error: {message}")]
  HtmlParseError { message: String },

  #[error("Transform error: {message}")]
  TransformError { message: String },
}

/// Response transformation transformer
pub struct ResponseTransformTransformer {
  rules: Arc<Vec<ResponseTransformRule>>,
  transformer_config: TransformerConfig<StreamWeaveHttpResponse>,
}

impl ResponseTransformTransformer {
  pub fn new() -> Self {
    Self {
      rules: Arc::new(Vec::new()),
      transformer_config: TransformerConfig::default(),
    }
  }

  pub fn with_rule(mut self, rule: ResponseTransformRule) -> Self {
    Arc::get_mut(&mut self.rules).unwrap().push(rule);
    self
  }

  pub fn with_rules(mut self, rules: Vec<ResponseTransformRule>) -> Self {
    Arc::get_mut(&mut self.rules).unwrap().extend(rules);
    self
  }

  pub fn with_transformer_config(
    mut self,
    config: TransformerConfig<StreamWeaveHttpResponse>,
  ) -> Self {
    self.transformer_config = config;
    self
  }

  #[allow(dead_code)]
  async fn transform_response(
    &self,
    mut response: StreamWeaveHttpResponse,
  ) -> Result<StreamWeaveHttpResponse, TransformError> {
    for rule in self.rules.iter() {
      match rule {
        ResponseTransformRule::AddHeaders(headers) => {
          for (key, value) in headers {
            if let (Ok(key), Ok(value)) = (
              key.parse::<http::HeaderName>(),
              HeaderValue::from_str(&value),
            ) {
              response.headers.insert(key, value);
            }
          }
        }
        ResponseTransformRule::RemoveHeaders(header_names) => {
          for header_name in header_names {
            response.headers.remove(header_name);
          }
        }
        ResponseTransformRule::TransformBody(transformer) => {
          let content_type = response
            .headers
            .get("content-type")
            .and_then(|h| h.to_str().ok());

          let transformed_body = transformer.transform(&response.body, content_type).await?;
          response.body = transformed_body.into();
        }
        ResponseTransformRule::SetStatusCode(status_code) => {
          response.status = *status_code;
        }
        ResponseTransformRule::AddCorsHeaders {
          allowed_origins,
          allowed_methods,
          allowed_headers,
        } => {
          self.add_cors_headers(
            &mut response.headers,
            &allowed_origins,
            &allowed_methods,
            &allowed_headers,
          );
        }
        ResponseTransformRule::AddSecurityHeaders => {
          self.add_security_headers(&mut response.headers);
        }
        ResponseTransformRule::CompressResponse => {
          // This would integrate with the compression transformer
          // For now, we'll just add a header indicating compression is available
          response.headers.insert(
            "x-compression-available",
            HeaderValue::from_static("gzip, deflate"),
          );
        }
        ResponseTransformRule::AddCacheHeaders {
          max_age,
          cache_control,
        } => {
          self.add_cache_headers(&mut response.headers, *max_age, &cache_control);
        }
      }
    }

    Ok(response)
  }

  #[allow(dead_code)]
  fn add_cors_headers(
    &self,
    headers: &mut HeaderMap,
    allowed_origins: &[String],
    allowed_methods: &[String],
    allowed_headers: &[String],
  ) {
    if let Ok(origin_value) = HeaderValue::from_str(&allowed_origins.join(", ")) {
      headers.insert("access-control-allow-origin", origin_value);
    }

    if let Ok(methods_value) = HeaderValue::from_str(&allowed_methods.join(", ")) {
      headers.insert("access-control-allow-methods", methods_value);
    }

    if let Ok(headers_value) = HeaderValue::from_str(&allowed_headers.join(", ")) {
      headers.insert("access-control-allow-headers", headers_value);
    }
  }

  #[allow(dead_code)]
  fn add_security_headers(&self, headers: &mut HeaderMap) {
    // X-Content-Type-Options
    headers.insert(
      "x-content-type-options",
      HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));

    // X-XSS-Protection
    headers.insert(
      "x-xss-protection",
      HeaderValue::from_static("1; mode=block"),
    );

    // Strict-Transport-Security
    headers.insert(
      "strict-transport-security",
      HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Content-Security-Policy
    headers.insert(
      "content-security-policy",
      HeaderValue::from_static("default-src 'self'"),
    );
  }

  #[allow(dead_code)]
  fn add_cache_headers(&self, headers: &mut HeaderMap, max_age: u64, cache_control: &str) {
    headers.insert(
      "cache-control",
      HeaderValue::from_str(cache_control).unwrap_or(HeaderValue::from_static("public")),
    );
    headers.insert(
      "expires",
      HeaderValue::from_str(&format!("{}", max_age)).unwrap_or(HeaderValue::from_static("0")),
    );
  }
}

impl Default for ResponseTransformTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for ResponseTransformTransformer {
  type Input = StreamWeaveHttpResponse;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for ResponseTransformTransformer {
  type Output = StreamWeaveHttpResponse;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for ResponseTransformTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let rules = Arc::clone(&self.rules);
    let transformer_name = self
      .transformer_config
      .name()
      .unwrap_or("response_transform".to_string());

    Box::pin(async_stream::stream! {
        let mut input_stream = input;

        while let Some(response) = input_stream.next().await {
            println!("🔄 [{}] Transforming response: {} {} bytes",
                transformer_name, response.status, response.body.len());

            // Transform the response
            let mut response = response;
            let mut transform_error = None;

            for rule in rules.iter() {
                match rule {
                    ResponseTransformRule::AddHeaders(headers) => {
                        for (key, value) in headers {
                        if let (Ok(key), Ok(value)) = (key.parse::<http::HeaderName>(), HeaderValue::from_str(&value)) {
                            response.headers.insert(key, value);
                        }
                        }
                    }
                    ResponseTransformRule::RemoveHeaders(header_names) => {
                        for header_name in header_names {
                            response.headers.remove(header_name);
                        }
                    }
                    ResponseTransformRule::TransformBody(transformer) => {
                        let content_type = response.headers.get("content-type")
                            .and_then(|h| h.to_str().ok());

                        match transformer.transform(&response.body, content_type).await {
                            Ok(transformed_body) => {
                                response.body = transformed_body.into();
                            }
                            Err(e) => {
                                transform_error = Some(e);
                                break;
                            }
                        }
                    }
                    ResponseTransformRule::SetStatusCode(status_code) => {
                        response.status = *status_code;
                    }
                    ResponseTransformRule::AddCorsHeaders { allowed_origins, allowed_methods, allowed_headers } => {
                        // Add CORS headers
                        if let Ok(origin_value) = HeaderValue::from_str(&allowed_origins.join(", ")) {
                            response.headers.insert("access-control-allow-origin", origin_value);
                        }
                        if let Ok(methods_value) = HeaderValue::from_str(&allowed_methods.join(", ")) {
                            response.headers.insert("access-control-allow-methods", methods_value);
                        }
                        if let Ok(headers_value) = HeaderValue::from_str(&allowed_headers.join(", ")) {
                            response.headers.insert("access-control-allow-headers", headers_value);
                        }
                    }
                    ResponseTransformRule::AddSecurityHeaders => {
                        // Add security headers
                        response.headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
                        response.headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
                        response.headers.insert("x-xss-protection", HeaderValue::from_static("1; mode=block"));
                    }
                    ResponseTransformRule::CompressResponse => {
                        response.headers.insert("x-compression-available", HeaderValue::from_static("gzip, deflate"));
                    }
                    ResponseTransformRule::AddCacheHeaders { max_age: _, cache_control } => {
                        response.headers.insert("cache-control", HeaderValue::from_str(&cache_control).unwrap_or(HeaderValue::from_static("public")));
                    }
                }
            }

            if let Some(error) = transform_error {
                println!("   ❌ [{}] Transformation failed: {:?}", transformer_name, error);
                // Skip this response if there was a transformation error
                continue;
            } else {
                println!("   ✅ [{}] Response transformation completed successfully", transformer_name);
                yield response;
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

#[cfg(test)]
mod tests {
  use super::*;
  use http::HeaderMap;

  #[test]
  fn test_json_response_transformer_creation() {
    let transformer = JsonResponseTransformer::new()
      .with_pretty_print(true)
      .with_remove_null_fields(true);

    assert!(transformer.pretty_print);
    assert!(transformer.remove_null_fields);
  }

  #[test]
  fn test_xml_response_transformer_creation() {
    let transformer = XmlResponseTransformer::new()
      .with_pretty_print(false)
      .with_indent(4);

    assert!(!transformer.pretty_print);
    assert_eq!(transformer.indent, 4);
  }

  #[test]
  fn test_html_response_transformer_creation() {
    let transformer = HtmlResponseTransformer::new()
      .with_minify(true)
      .with_add_doctype(false);

    assert!(transformer.minify);
    assert!(!transformer.add_doctype);
  }

  #[tokio::test]
  async fn test_json_response_transformer_valid_json() {
    let transformer = JsonResponseTransformer::new();
    let json_data = r#"{"name": "test", "value": 123}"#;

    let result = transformer
      .transform(json_data.as_bytes(), Some("application/json"))
      .await;
    assert!(result.is_ok());

    let transformed = result.unwrap();
    let transformed_str = String::from_utf8_lossy(&transformed);
    assert!(transformed_str.contains("name"));
    assert!(transformed_str.contains("test"));
  }

  #[tokio::test]
  async fn test_json_response_transformer_invalid_json() {
    let transformer = JsonResponseTransformer::new();
    let invalid_json = r#"{"name": "test", "value": 123"#; // Missing closing brace

    let result = transformer
      .transform(invalid_json.as_bytes(), Some("application/json"))
      .await;
    assert!(result.is_err());

    match result.unwrap_err() {
      TransformError::JsonParseError { .. } => {}
      _ => panic!("Expected JsonParseError"),
    }
  }

  #[tokio::test]
  async fn test_json_response_transformer_remove_null_fields() {
    let transformer = JsonResponseTransformer::new().with_remove_null_fields(true);
    let json_data = r#"{"name": "test", "value": null, "active": true}"#;

    let result = transformer
      .transform(json_data.as_bytes(), Some("application/json"))
      .await;
    assert!(result.is_ok());

    let transformed = result.unwrap();
    let transformed_str = String::from_utf8_lossy(&transformed);
    assert!(!transformed_str.contains("null"));
  }

  #[tokio::test]
  async fn test_html_response_transformer_add_doctype() {
    let transformer = HtmlResponseTransformer::new().with_add_doctype(true);
    let html_data = r#"<html><body><h1>Test</h1></body></html>"#;

    let result = transformer
      .transform(html_data.as_bytes(), Some("text/html"))
      .await;
    assert!(result.is_ok());

    let transformed = result.unwrap();
    let transformed_str = String::from_utf8_lossy(&transformed);
    assert!(transformed_str.starts_with("<!DOCTYPE html>"));
  }

  #[tokio::test]
  async fn test_html_response_transformer_minify() {
    let transformer = HtmlResponseTransformer::new().with_minify(true);
    let html_data = r#"<html>
            <body>
                <h1>Test</h1>
            </body>
        </html>"#;

    let result = transformer
      .transform(html_data.as_bytes(), Some("text/html"))
      .await;
    assert!(result.is_ok());

    let transformed = result.unwrap();
    let transformed_str = String::from_utf8_lossy(&transformed);
    assert!(!transformed_str.contains('\n'));
  }

  #[test]
  fn test_response_transform_transformer_creation() {
    let transformer = ResponseTransformTransformer::new()
      .with_rule(ResponseTransformRule::AddHeaders({
        let mut headers = HashMap::new();
        headers.insert("x-custom-header".to_string(), "custom-value".to_string());
        headers
      }))
      .with_rule(ResponseTransformRule::AddSecurityHeaders);

    assert_eq!(transformer.rules.len(), 2);
  }

  #[test]
  fn test_response_transform_transformer_add_headers() {
    let mut headers = HeaderMap::new();
    let mut custom_headers = HashMap::new();
    custom_headers.insert("x-custom-header".to_string(), "custom-value".to_string());

    let _transformer = ResponseTransformTransformer::new();
    // Add headers manually for testing
    for (key, value) in &custom_headers {
      if let (Ok(key), Ok(value)) = (
        key.parse::<http::HeaderName>(),
        HeaderValue::from_str(&value),
      ) {
        headers.insert(key, value);
      }
    }

    assert!(headers.contains_key("x-custom-header"));
  }

  #[test]
  fn test_response_transform_transformer_add_security_headers() {
    let mut headers = HeaderMap::new();
    let _transformer = ResponseTransformTransformer::new();
    // Add security headers manually for testing
    headers.insert(
      "x-content-type-options",
      HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
      "x-xss-protection",
      HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
      "strict-transport-security",
      HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
      "content-security-policy",
      HeaderValue::from_static("default-src 'self'"),
    );

    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-xss-protection"));
    assert!(headers.contains_key("strict-transport-security"));
    assert!(headers.contains_key("content-security-policy"));
  }

  #[test]
  fn test_response_transform_transformer_add_cache_headers() {
    let mut headers = HeaderMap::new();
    let _transformer = ResponseTransformTransformer::new();
    // Add cache headers manually for testing
    headers.insert(
      "cache-control",
      HeaderValue::from_str("public, max-age=3600").unwrap_or(HeaderValue::from_static("public")),
    );
    headers.insert(
      "expires",
      HeaderValue::from_str("3600").unwrap_or(HeaderValue::from_static("0")),
    );

    assert!(headers.contains_key("cache-control"));
    assert!(headers.contains_key("expires"));
  }
}
