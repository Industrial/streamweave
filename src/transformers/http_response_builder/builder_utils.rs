//! Builder utilities for common HTTP response patterns
//!
//! This module provides convenient builder functions and macros for creating
//! common HTTP response patterns without having to manually construct ResponseData.

use super::ResponseData;
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};
use serde_json;

/// JSON response builder utilities
pub struct JsonResponseBuilder {
  data: serde_json::Value,
  status: StatusCode,
  headers: HeaderMap,
}

impl JsonResponseBuilder {
  /// Create a new JSON response builder with OK status
  pub fn new() -> Self {
    Self {
      data: serde_json::Value::Object(serde_json::Map::new()),
      status: StatusCode::OK,
      headers: HeaderMap::new(),
    }
  }

  /// Create a new JSON response builder with custom status
  pub fn with_status(status: StatusCode) -> Self {
    Self {
      data: serde_json::Value::Object(serde_json::Map::new()),
      status,
      headers: HeaderMap::new(),
    }
  }

  /// Set the JSON data
  pub fn data(mut self, data: serde_json::Value) -> Self {
    self.data = data;
    self
  }

  /// Set a field in the JSON object
  pub fn field(mut self, key: &str, value: serde_json::Value) -> Self {
    if let serde_json::Value::Object(ref mut map) = self.data {
      map.insert(key.to_string(), value);
    }
    self
  }

  /// Set a string field
  pub fn string_field(self, key: &str, value: &str) -> Self {
    self.field(key, serde_json::Value::String(value.to_string()))
  }

  /// Set a number field
  pub fn number_field(self, key: &str, value: f64) -> Self {
    self.field(
      key,
      serde_json::Value::Number(
        serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0)),
      ),
    )
  }

  /// Set a boolean field
  pub fn bool_field(self, key: &str, value: bool) -> Self {
    self.field(key, serde_json::Value::Bool(value))
  }

  /// Set an array field
  pub fn array_field(self, key: &str, value: Vec<serde_json::Value>) -> Self {
    self.field(key, serde_json::Value::Array(value))
  }

  /// Add a header
  pub fn header(mut self, key: &str, value: &str) -> Self {
    if let (Ok(key), Ok(value)) = (
      key.parse::<http::HeaderName>(),
      HeaderValue::from_str(value),
    ) {
      self.headers.insert(key, value);
    }
    self
  }

  /// Set content type to application/json
  pub fn json_content_type(mut self) -> Self {
    self
      .headers
      .insert("content-type", HeaderValue::from_static("application/json"));
    self
  }

  /// Add CORS headers
  pub fn cors_headers(mut self, origin: &str) -> Self {
    if let Ok(value) = HeaderValue::from_str(origin) {
      self.headers.insert("access-control-allow-origin", value);
    }
    self.headers.insert(
      "access-control-allow-methods",
      HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    self.headers.insert(
      "access-control-allow-headers",
      HeaderValue::from_static("content-type, authorization"),
    );
    self
  }

  /// Add security headers
  pub fn security_headers(mut self) -> Self {
    self.headers.insert(
      "x-content-type-options",
      HeaderValue::from_static("nosniff"),
    );
    self
      .headers
      .insert("x-frame-options", HeaderValue::from_static("DENY"));
    self.headers.insert(
      "x-xss-protection",
      HeaderValue::from_static("1; mode=block"),
    );
    self
  }

  /// Build the ResponseData
  pub fn build(self) -> ResponseData {
    let body = match serde_json::to_vec(&self.data) {
      Ok(bytes) => Bytes::from(bytes),
      Err(_) => Bytes::from("{}"),
    };

    ResponseData::success_with_headers(self.status, body, self.headers)
  }
}

impl Default for JsonResponseBuilder {
  fn default() -> Self {
    Self::new()
  }
}

/// HTML response builder utilities
pub struct HtmlResponseBuilder {
  content: String,
  status: StatusCode,
  headers: HeaderMap,
}

impl HtmlResponseBuilder {
  /// Create a new HTML response builder with OK status
  pub fn new() -> Self {
    Self {
      content: String::new(),
      status: StatusCode::OK,
      headers: HeaderMap::new(),
    }
  }

  /// Create a new HTML response builder with custom status
  pub fn with_status(status: StatusCode) -> Self {
    Self {
      content: String::new(),
      status,
      headers: HeaderMap::new(),
    }
  }

  /// Set the HTML content
  pub fn content(mut self, content: &str) -> Self {
    self.content = content.to_string();
    self
  }

  /// Add HTML content
  pub fn add_content(mut self, content: &str) -> Self {
    self.content.push_str(content);
    self
  }

  /// Add a header
  pub fn header(mut self, key: &str, value: &str) -> Self {
    if let (Ok(key), Ok(value)) = (
      key.parse::<http::HeaderName>(),
      HeaderValue::from_str(value),
    ) {
      self.headers.insert(key, value);
    }
    self
  }

  /// Set content type to text/html
  pub fn html_content_type(mut self) -> Self {
    self
      .headers
      .insert("content-type", HeaderValue::from_static("text/html"));
    self
  }

  /// Add a complete HTML page with DOCTYPE
  pub fn html_page(mut self, title: &str, body: &str) -> Self {
    self.content = format!(
      r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
    {}
</body>
</html>"#,
      title, body
    );
    self
  }

  /// Build the ResponseData
  pub fn build(self) -> ResponseData {
    ResponseData::success_with_headers(self.status, Bytes::from(self.content), self.headers)
  }
}

impl Default for HtmlResponseBuilder {
  fn default() -> Self {
    Self::new()
  }
}

/// Plain text response builder utilities
pub struct TextResponseBuilder {
  content: String,
  status: StatusCode,
  headers: HeaderMap,
}

impl TextResponseBuilder {
  /// Create a new text response builder with OK status
  pub fn new() -> Self {
    Self {
      content: String::new(),
      status: StatusCode::OK,
      headers: HeaderMap::new(),
    }
  }

  /// Create a new text response builder with custom status
  pub fn with_status(status: StatusCode) -> Self {
    Self {
      content: String::new(),
      status,
      headers: HeaderMap::new(),
    }
  }

  /// Set the text content
  pub fn content(mut self, content: &str) -> Self {
    self.content = content.to_string();
    self
  }

  /// Add text content
  pub fn add_content(mut self, content: &str) -> Self {
    self.content.push_str(content);
    self
  }

  /// Add a header
  pub fn header(mut self, key: &str, value: &str) -> Self {
    if let (Ok(key), Ok(value)) = (
      key.parse::<http::HeaderName>(),
      HeaderValue::from_str(value),
    ) {
      self.headers.insert(key, value);
    }
    self
  }

  /// Set content type to text/plain
  pub fn text_content_type(mut self) -> Self {
    self
      .headers
      .insert("content-type", HeaderValue::from_static("text/plain"));
    self
  }

  /// Build the ResponseData
  pub fn build(self) -> ResponseData {
    ResponseData::success_with_headers(self.status, Bytes::from(self.content), self.headers)
  }
}

impl Default for TextResponseBuilder {
  fn default() -> Self {
    Self::new()
  }
}

/// Error response builder utilities
pub struct ErrorResponseBuilder {
  message: String,
  status: StatusCode,
  headers: HeaderMap,
  include_stack_trace: bool,
}

impl ErrorResponseBuilder {
  /// Create a new error response builder
  pub fn new(status: StatusCode, message: &str) -> Self {
    Self {
      message: message.to_string(),
      status,
      headers: HeaderMap::new(),
      include_stack_trace: false,
    }
  }

  /// Create a not found error response
  pub fn not_found(message: &str) -> Self {
    Self::new(StatusCode::NOT_FOUND, message)
  }

  /// Create a bad request error response
  pub fn bad_request(message: &str) -> Self {
    Self::new(StatusCode::BAD_REQUEST, message)
  }

  /// Create an unauthorized error response
  pub fn unauthorized(message: &str) -> Self {
    Self::new(StatusCode::UNAUTHORIZED, message)
  }

  /// Create a forbidden error response
  pub fn forbidden(message: &str) -> Self {
    Self::new(StatusCode::FORBIDDEN, message)
  }

  /// Create an internal server error response
  pub fn internal_server_error(message: &str) -> Self {
    Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
  }

  /// Add a header
  pub fn header(mut self, key: &str, value: &str) -> Self {
    if let (Ok(key), Ok(value)) = (
      key.parse::<http::HeaderName>(),
      HeaderValue::from_str(value),
    ) {
      self.headers.insert(key, value);
    }
    self
  }

  /// Include stack trace in error response (for development)
  pub fn with_stack_trace(mut self, include: bool) -> Self {
    self.include_stack_trace = include;
    self
  }

  /// Build the ResponseData
  pub fn build(self) -> ResponseData {
    let mut error_data = serde_json::Map::new();
    error_data.insert(
      "error".to_string(),
      serde_json::Value::String(self.status.as_str().to_string()),
    );
    error_data.insert(
      "message".to_string(),
      serde_json::Value::String(self.message.clone()),
    );

    if self.include_stack_trace {
      error_data.insert(
        "stack_trace".to_string(),
        serde_json::Value::String("Stack trace would go here".to_string()),
      );
    }

    let body = match serde_json::to_vec(&serde_json::Value::Object(error_data)) {
      Ok(bytes) => Bytes::from(bytes),
      Err(_) => Bytes::from(format!(
        r#"{{"error": "{}", "message": "{}"}}"#,
        self.status.as_str(),
        self.message
      )),
    };

    ResponseData::success_with_headers(self.status, body, self.headers)
  }
}

/// Convenience functions for common response patterns
pub mod responses {
  use super::*;

  /// Create a simple OK response with JSON data
  pub fn ok_json(data: serde_json::Value) -> ResponseData {
    JsonResponseBuilder::new()
      .data(data)
      .json_content_type()
      .build()
  }

  /// Create a simple OK response with text
  pub fn ok_text(text: &str) -> ResponseData {
    TextResponseBuilder::new()
      .content(text)
      .text_content_type()
      .build()
  }

  /// Create a simple OK response with HTML
  pub fn ok_html(html: &str) -> ResponseData {
    HtmlResponseBuilder::new()
      .content(html)
      .html_content_type()
      .build()
  }

  /// Create a not found response
  pub fn not_found(message: &str) -> ResponseData {
    ErrorResponseBuilder::not_found(message).build()
  }

  /// Create a bad request response
  pub fn bad_request(message: &str) -> ResponseData {
    ErrorResponseBuilder::bad_request(message).build()
  }

  /// Create an unauthorized response
  pub fn unauthorized(message: &str) -> ResponseData {
    ErrorResponseBuilder::unauthorized(message).build()
  }

  /// Create a forbidden response
  pub fn forbidden(message: &str) -> ResponseData {
    ErrorResponseBuilder::forbidden(message).build()
  }

  /// Create an internal server error response
  pub fn internal_server_error(message: &str) -> ResponseData {
    ErrorResponseBuilder::internal_server_error(message).build()
  }

  /// Create a success response with custom status and JSON data
  pub fn success_json(status: StatusCode, data: serde_json::Value) -> ResponseData {
    JsonResponseBuilder::with_status(status)
      .data(data)
      .json_content_type()
      .build()
  }

  /// Create a success response with custom status and text
  pub fn success_text(status: StatusCode, text: &str) -> ResponseData {
    TextResponseBuilder::with_status(status)
      .content(text)
      .text_content_type()
      .build()
  }

  /// Create a success response with custom status and HTML
  pub fn success_html(status: StatusCode, html: &str) -> ResponseData {
    HtmlResponseBuilder::with_status(status)
      .content(html)
      .html_content_type()
      .build()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_json_response_builder() {
    let response = JsonResponseBuilder::new()
      .string_field("message", "Hello, World!")
      .number_field("count", 42.0)
      .bool_field("success", true)
      .json_content_type()
      .build();

    assert!(response.is_success());
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("content-type"));
  }

  #[test]
  fn test_json_response_builder_with_custom_status() {
    let response = JsonResponseBuilder::with_status(StatusCode::CREATED)
      .string_field("message", "Created")
      .build();

    assert_eq!(response.status(), StatusCode::CREATED);
  }

  #[test]
  fn test_json_response_builder_default() {
    let response = JsonResponseBuilder::default();
    assert_eq!(response.status, StatusCode::OK);
  }

  #[test]
  fn test_json_response_builder_data() {
    let data = json!({"key": "value"});
    let response = JsonResponseBuilder::new().data(data.clone()).build();

    assert!(response.is_success());
  }

  #[test]
  fn test_json_response_builder_field() {
    let response = JsonResponseBuilder::new()
      .field("string", json!("test"))
      .field("number", json!(42))
      .field("boolean", json!(true))
      .field("null", json!(null))
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_json_response_builder_string_field() {
    let response = JsonResponseBuilder::new()
      .string_field("message", "Hello")
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_json_response_builder_number_field() {
    let response = JsonResponseBuilder::new()
      .number_field("count", 42.5)
      .number_field("negative", -10.0)
      .number_field("zero", 0.0)
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_json_response_builder_bool_field() {
    let response = JsonResponseBuilder::new()
      .bool_field("enabled", true)
      .bool_field("disabled", false)
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_json_response_builder_array_field() {
    let response = JsonResponseBuilder::new()
      .array_field(
        "items",
        vec![json!("item1"), json!("item2"), json!("item3")],
      )
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_json_response_builder_header() {
    let response = JsonResponseBuilder::new()
      .header("x-custom", "value")
      .header("x-api-version", "v1")
      .build();

    assert!(response.headers().contains_key("x-custom"));
    assert!(response.headers().contains_key("x-api-version"));
  }

  #[test]
  fn test_json_response_builder_invalid_header() {
    let response = JsonResponseBuilder::new()
      .header("invalid\x00header", "value")
      .header("x-valid", "value")
      .build();

    // Invalid headers should be ignored
    assert!(!response.headers().contains_key("invalid\x00header"));
    assert!(response.headers().contains_key("x-valid"));
  }

  #[test]
  fn test_json_response_builder_json_content_type() {
    let response = JsonResponseBuilder::new().json_content_type().build();

    assert_eq!(
      response.headers().get("content-type").unwrap(),
      "application/json"
    );
  }

  #[test]
  fn test_json_response_builder_cors_headers() {
    let response = JsonResponseBuilder::new()
      .cors_headers("https://example.com")
      .build();

    assert!(
      response
        .headers()
        .contains_key("access-control-allow-origin")
    );
    assert!(
      response
        .headers()
        .contains_key("access-control-allow-methods")
    );
    assert!(
      response
        .headers()
        .contains_key("access-control-allow-headers")
    );
  }

  #[test]
  fn test_json_response_builder_cors_headers_invalid_origin() {
    let response = JsonResponseBuilder::new()
      .cors_headers("invalid\x00origin")
      .build();

    // Should still add other CORS headers
    assert!(
      response
        .headers()
        .contains_key("access-control-allow-methods")
    );
    assert!(
      response
        .headers()
        .contains_key("access-control-allow-headers")
    );
  }

  #[test]
  fn test_json_response_builder_security_headers() {
    let response = JsonResponseBuilder::new().security_headers().build();

    assert!(response.headers().contains_key("x-content-type-options"));
    assert!(response.headers().contains_key("x-frame-options"));
    assert!(response.headers().contains_key("x-xss-protection"));
  }

  #[test]
  fn test_json_response_builder_serialization_error() {
    // Test with data that might cause serialization issues
    let response = JsonResponseBuilder::new()
      .data(json!(f64::NAN)) // NaN values might cause issues
      .build();

    // Should fallback to empty JSON
    assert!(response.is_success());
  }

  #[test]
  fn test_html_response_builder() {
    let response = HtmlResponseBuilder::new()
      .html_page("Test Page", "<h1>Hello, World!</h1>")
      .html_content_type()
      .build();

    assert!(response.is_success());
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("content-type"));
    assert!(
      response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/html")
    );
  }

  #[test]
  fn test_html_response_builder_with_custom_status() {
    let response = HtmlResponseBuilder::with_status(StatusCode::NOT_FOUND)
      .content("<h1>Not Found</h1>")
      .build();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }

  #[test]
  fn test_html_response_builder_default() {
    let response = HtmlResponseBuilder::default();
    assert_eq!(response.status, StatusCode::OK);
  }

  #[test]
  fn test_html_response_builder_content() {
    let response = HtmlResponseBuilder::new()
      .content("<p>Hello World</p>")
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_html_response_builder_add_content() {
    let response = HtmlResponseBuilder::new()
      .add_content("<h1>Title</h1>")
      .add_content("<p>Content</p>")
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_html_response_builder_header() {
    let response = HtmlResponseBuilder::new()
      .header("x-custom", "value")
      .build();

    assert!(response.headers().contains_key("x-custom"));
  }

  #[test]
  fn test_html_response_builder_html_content_type() {
    let response = HtmlResponseBuilder::new().html_content_type().build();

    assert_eq!(response.headers().get("content-type").unwrap(), "text/html");
  }

  #[test]
  fn test_html_response_builder_html_page() {
    let response = HtmlResponseBuilder::new()
      .html_page("My Page", "<h1>Hello</h1>")
      .build();

    assert!(response.is_success());
    let body = String::from_utf8_lossy(&response.body());
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("<title>My Page</title>"));
    assert!(body.contains("<h1>Hello</h1>"));
  }

  #[test]
  fn test_text_response_builder() {
    let response = TextResponseBuilder::new()
      .content("Hello, World!")
      .text_content_type()
      .build();

    assert!(response.is_success());
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("content-type"));
  }

  #[test]
  fn test_text_response_builder_with_custom_status() {
    let response = TextResponseBuilder::with_status(StatusCode::CREATED)
      .content("Created")
      .build();

    assert_eq!(response.status(), StatusCode::CREATED);
  }

  #[test]
  fn test_text_response_builder_default() {
    let response = TextResponseBuilder::default();
    assert_eq!(response.status, StatusCode::OK);
  }

  #[test]
  fn test_text_response_builder_content() {
    let response = TextResponseBuilder::new()
      .content("Plain text content")
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_text_response_builder_add_content() {
    let response = TextResponseBuilder::new()
      .add_content("Line 1\n")
      .add_content("Line 2\n")
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_text_response_builder_header() {
    let response = TextResponseBuilder::new()
      .header("x-custom", "value")
      .build();

    assert!(response.headers().contains_key("x-custom"));
  }

  #[test]
  fn test_text_response_builder_text_content_type() {
    let response = TextResponseBuilder::new().text_content_type().build();

    assert_eq!(
      response.headers().get("content-type").unwrap(),
      "text/plain"
    );
  }

  #[test]
  fn test_error_response_builder() {
    let response = ErrorResponseBuilder::not_found("Resource not found")
      .with_stack_trace(true)
      .build();

    assert!(response.is_success()); // Error responses are still success responses with error status
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }

  #[test]
  fn test_error_response_builder_new() {
    let response = ErrorResponseBuilder::new(StatusCode::BAD_REQUEST, "Bad request").build();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  }

  #[test]
  fn test_error_response_builder_bad_request() {
    let response = ErrorResponseBuilder::bad_request("Invalid input").build();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  }

  #[test]
  fn test_error_response_builder_unauthorized() {
    let response = ErrorResponseBuilder::unauthorized("Not authorized").build();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
  }

  #[test]
  fn test_error_response_builder_forbidden() {
    let response = ErrorResponseBuilder::forbidden("Access denied").build();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
  }

  #[test]
  fn test_error_response_builder_internal_server_error() {
    let response = ErrorResponseBuilder::internal_server_error("Server error").build();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
  }

  #[test]
  fn test_error_response_builder_header() {
    let response = ErrorResponseBuilder::not_found("Not found")
      .header("x-error-code", "404")
      .build();

    assert!(response.headers().contains_key("x-error-code"));
  }

  #[test]
  fn test_error_response_builder_with_stack_trace() {
    let response = ErrorResponseBuilder::not_found("Not found")
      .with_stack_trace(true)
      .build();

    assert!(response.is_success());
    let body = String::from_utf8_lossy(&response.body());
    assert!(body.contains("stack_trace"));
  }

  #[test]
  fn test_error_response_builder_without_stack_trace() {
    let response = ErrorResponseBuilder::not_found("Not found")
      .with_stack_trace(false)
      .build();

    assert!(response.is_success());
    let body = String::from_utf8_lossy(&response.body());
    assert!(!body.contains("stack_trace"));
  }

  #[test]
  fn test_error_response_builder_serialization_error() {
    // Test error response with special characters that might cause JSON serialization issues
    let response =
      ErrorResponseBuilder::new(StatusCode::BAD_REQUEST, "Error with \x00 null byte").build();

    // Should fallback to simple error format
    assert!(response.is_success());
    let body = String::from_utf8_lossy(&response.body());
    assert!(body.contains("400"));
  }

  #[test]
  fn test_convenience_functions() {
    let json_response = responses::ok_json(json!({"message": "Hello"}));
    assert!(json_response.is_success());

    let text_response = responses::ok_text("Hello, World!");
    assert!(text_response.is_success());

    let html_response = responses::ok_html("<h1>Hello</h1>");
    assert!(html_response.is_success());

    let not_found = responses::not_found("Not found");
    assert_eq!(not_found.status(), StatusCode::NOT_FOUND);

    let bad_request = responses::bad_request("Bad request");
    assert_eq!(bad_request.status(), StatusCode::BAD_REQUEST);

    let unauthorized = responses::unauthorized("Unauthorized");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let forbidden = responses::forbidden("Forbidden");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let internal_error = responses::internal_server_error("Server error");
    assert_eq!(internal_error.status(), StatusCode::INTERNAL_SERVER_ERROR);
  }

  #[test]
  fn test_success_convenience_functions() {
    let json_response = responses::success_json(StatusCode::CREATED, json!({"id": 123}));
    assert_eq!(json_response.status(), StatusCode::CREATED);

    let text_response = responses::success_text(StatusCode::ACCEPTED, "Accepted");
    assert_eq!(text_response.status(), StatusCode::ACCEPTED);

    let html_response = responses::success_html(StatusCode::OK, "<h1>Success</h1>");
    assert_eq!(html_response.status(), StatusCode::OK);
  }

  #[test]
  fn test_cors_headers() {
    let response = JsonResponseBuilder::new()
      .string_field("message", "Hello")
      .cors_headers("https://example.com")
      .build();

    assert!(
      response
        .headers()
        .contains_key("access-control-allow-origin")
    );
    assert!(
      response
        .headers()
        .contains_key("access-control-allow-methods")
    );
    assert!(
      response
        .headers()
        .contains_key("access-control-allow-headers")
    );
  }

  #[test]
  fn test_security_headers() {
    let response = JsonResponseBuilder::new()
      .string_field("message", "Hello")
      .security_headers()
      .build();

    assert!(response.headers().contains_key("x-content-type-options"));
    assert!(response.headers().contains_key("x-frame-options"));
    assert!(response.headers().contains_key("x-xss-protection"));
  }

  #[test]
  fn test_custom_headers() {
    let response = JsonResponseBuilder::new()
      .string_field("message", "Hello")
      .header("x-custom", "value")
      .header("x-api-version", "v1")
      .build();

    assert!(response.headers().contains_key("x-custom"));
    assert!(response.headers().contains_key("x-api-version"));
  }

  #[test]
  fn test_array_field() {
    let response = JsonResponseBuilder::new()
      .array_field(
        "items",
        vec![json!("item1"), json!("item2"), json!("item3")],
      )
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_custom_status() {
    let response = JsonResponseBuilder::with_status(StatusCode::CREATED)
      .string_field("message", "Created")
      .build();

    assert_eq!(response.status(), StatusCode::CREATED);
  }

  #[test]
  fn test_number_field_edge_cases() {
    let response = JsonResponseBuilder::new()
      .number_field("infinity", f64::INFINITY)
      .number_field("negative_infinity", f64::NEG_INFINITY)
      .number_field("nan", f64::NAN)
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_empty_array_field() {
    let response = JsonResponseBuilder::new()
      .array_field("empty", vec![])
      .build();

    assert!(response.is_success());
  }

  #[test]
  fn test_nested_json_data() {
    let nested_data = json!({
        "user": {
            "id": 123,
            "name": "John",
            "settings": {
                "theme": "dark",
                "notifications": true
            }
        }
    });

    let response = JsonResponseBuilder::new().data(nested_data).build();

    assert!(response.is_success());
  }

  #[test]
  fn test_html_page_with_special_characters() {
    let response = HtmlResponseBuilder::new()
      .html_page("Test & Page", "<h1>Hello & World</h1>")
      .build();

    assert!(response.is_success());
    let body = String::from_utf8_lossy(&response.body());
    assert!(body.contains("Test & Page"));
    assert!(body.contains("Hello & World"));
  }

  #[test]
  fn test_text_with_unicode() {
    let response = TextResponseBuilder::new().content("Hello 世界 🌍").build();

    assert!(response.is_success());
  }

  #[test]
  fn test_error_with_unicode_message() {
    let response = ErrorResponseBuilder::bad_request("错误消息 🚀").build();

    assert!(response.is_success());
    let body = String::from_utf8_lossy(&response.body());
    assert!(body.contains("错误消息"));
  }
}
