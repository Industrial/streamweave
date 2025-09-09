use bytes::Bytes;
use http::{HeaderMap, StatusCode};

/// Response data that can be converted to HTTP responses
/// This enum provides a clean abstraction for different types of responses
/// that can be processed by the HttpResponseBuilderTransformer
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseData {
  /// Success response with status, body, and headers
  Success {
    status: StatusCode,
    body: Bytes,
    headers: HeaderMap,
  },
  /// Error response with status and message
  Error { status: StatusCode, message: String },
}

impl ResponseData {
  /// Create a success response with OK status
  pub fn ok(body: Bytes) -> Self {
    Self::Success {
      status: StatusCode::OK,
      body,
      headers: HeaderMap::new(),
    }
  }

  /// Create a success response with custom status
  pub fn success(status: StatusCode, body: Bytes) -> Self {
    Self::Success {
      status,
      body,
      headers: HeaderMap::new(),
    }
  }

  /// Create a success response with headers
  pub fn success_with_headers(status: StatusCode, body: Bytes, headers: HeaderMap) -> Self {
    Self::Success {
      status,
      body,
      headers,
    }
  }

  /// Create an error response
  pub fn error(status: StatusCode, message: String) -> Self {
    Self::Error { status, message }
  }

  /// Create a not found error response
  pub fn not_found(message: String) -> Self {
    Self::Error {
      status: StatusCode::NOT_FOUND,
      message,
    }
  }

  /// Create an internal server error response
  pub fn internal_server_error(message: String) -> Self {
    Self::Error {
      status: StatusCode::INTERNAL_SERVER_ERROR,
      message,
    }
  }

  /// Create a bad request error response
  pub fn bad_request(message: String) -> Self {
    Self::Error {
      status: StatusCode::BAD_REQUEST,
      message,
    }
  }

  /// Create an unauthorized error response
  pub fn unauthorized(message: String) -> Self {
    Self::Error {
      status: StatusCode::UNAUTHORIZED,
      message,
    }
  }

  /// Create a forbidden error response
  pub fn forbidden(message: String) -> Self {
    Self::Error {
      status: StatusCode::FORBIDDEN,
      message,
    }
  }

  /// Get the status code of the response
  pub fn status(&self) -> StatusCode {
    match self {
      ResponseData::Success { status, .. } => *status,
      ResponseData::Error { status, .. } => *status,
    }
  }

  /// Get the headers of the response
  pub fn headers(&self) -> &HeaderMap {
    match self {
      ResponseData::Success { headers, .. } => headers,
      ResponseData::Error { .. } => {
        // Return a static empty HeaderMap for error responses
        static EMPTY_HEADERS: std::sync::LazyLock<HeaderMap> =
          std::sync::LazyLock::new(|| HeaderMap::new());
        &EMPTY_HEADERS
      }
    }
  }

  /// Get the body of the response
  pub fn body(&self) -> &Bytes {
    match self {
      ResponseData::Success { body, .. } => body,
      ResponseData::Error { .. } => {
        // Return empty bytes for error responses
        static EMPTY_BODY: std::sync::LazyLock<Bytes> = std::sync::LazyLock::new(|| Bytes::new());
        &EMPTY_BODY
      }
    }
  }

  /// Check if this is an error response
  pub fn is_error(&self) -> bool {
    matches!(self, ResponseData::Error { .. })
  }

  /// Check if this is a success response
  pub fn is_success(&self) -> bool {
    matches!(self, ResponseData::Success { .. })
  }
}

impl Default for ResponseData {
  fn default() -> Self {
    Self::ok(Bytes::new())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_success_response_creation() {
    let response = ResponseData::ok(Bytes::from("Hello, World!"));
    assert!(response.is_success());
    assert!(!response.is_error());
    assert_eq!(response.status(), StatusCode::OK);
  }

  #[test]
  fn test_error_response_creation() {
    let response = ResponseData::error(StatusCode::NOT_FOUND, "Not found".to_string());
    assert!(!response.is_success());
    assert!(response.is_error());
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
  }

  #[test]
  fn test_response_with_headers() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    let response = ResponseData::success_with_headers(StatusCode::OK, Bytes::from("{}"), headers);

    assert!(response.headers().contains_key("content-type"));
  }

  #[test]
  fn test_convenience_methods() {
    let not_found = ResponseData::not_found("Resource not found".to_string());
    assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
    assert!(not_found.is_error());

    let internal_error = ResponseData::internal_server_error("Server error".to_string());
    assert_eq!(internal_error.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(internal_error.is_error());

    let bad_request = ResponseData::bad_request("Invalid request".to_string());
    assert_eq!(bad_request.status(), StatusCode::BAD_REQUEST);
    assert!(bad_request.is_error());

    let unauthorized = ResponseData::unauthorized("Unauthorized".to_string());
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    assert!(unauthorized.is_error());

    let forbidden = ResponseData::forbidden("Forbidden".to_string());
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    assert!(forbidden.is_error());
  }

  #[test]
  fn test_default_response() {
    let response = ResponseData::default();
    assert!(response.is_success());
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().len(), 0);
  }

  #[test]
  fn test_success_with_custom_status() {
    let response = ResponseData::success(StatusCode::CREATED, Bytes::from("Created"));
    assert!(response.is_success());
    assert!(!response.is_error());
    assert_eq!(response.status(), StatusCode::CREATED);
  }

  #[test]
  fn test_success_with_headers_and_custom_status() {
    let mut headers = HeaderMap::new();
    headers.insert("location", "/api/users/123".parse().unwrap());

    let response = ResponseData::success_with_headers(
      StatusCode::CREATED,
      Bytes::from("{\"id\": 123}"),
      headers,
    );

    assert!(response.is_success());
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(response.headers().contains_key("location"));
  }

  #[test]
  fn test_error_response_headers() {
    let error_response = ResponseData::error(StatusCode::BAD_REQUEST, "Bad request".to_string());
    assert!(error_response.is_error());
    assert_eq!(error_response.status(), StatusCode::BAD_REQUEST);

    // Error responses should return empty headers
    assert_eq!(error_response.headers().len(), 0);
  }

  #[test]
  fn test_response_data_clone() {
    let response1 = ResponseData::ok(Bytes::from("test"));
    let response2 = response1.clone();

    assert_eq!(response1.status(), response2.status());
    assert_eq!(response1.is_success(), response2.is_success());
    assert_eq!(response1.is_error(), response2.is_error());
  }

  #[test]
  fn test_response_data_debug() {
    let response = ResponseData::ok(Bytes::from("test"));
    let debug_str = format!("{:?}", response);
    assert!(debug_str.contains("Success"));
  }

  #[test]
  fn test_response_data_partial_eq() {
    let response1 = ResponseData::ok(Bytes::from("test"));
    let response2 = ResponseData::ok(Bytes::from("test"));
    let response3 = ResponseData::ok(Bytes::from("different"));

    assert_eq!(response1, response2);
    assert_ne!(response1, response3);
  }

  #[test]
  fn test_all_status_codes() {
    let status_codes = vec![
      StatusCode::OK,
      StatusCode::CREATED,
      StatusCode::NO_CONTENT,
      StatusCode::BAD_REQUEST,
      StatusCode::UNAUTHORIZED,
      StatusCode::FORBIDDEN,
      StatusCode::NOT_FOUND,
      StatusCode::INTERNAL_SERVER_ERROR,
    ];

    for status in status_codes {
      let success_response = ResponseData::success(status, Bytes::from("test"));
      assert_eq!(success_response.status(), status);

      let error_response = ResponseData::error(status, "error".to_string());
      assert_eq!(error_response.status(), status);
    }
  }

  #[test]
  fn test_empty_bytes() {
    let response = ResponseData::ok(Bytes::new());
    assert!(response.is_success());
    assert_eq!(response.status(), StatusCode::OK);
  }

  #[test]
  fn test_large_bytes() {
    let large_data = Bytes::from(vec![0u8; 10000]);
    let response = ResponseData::ok(large_data.clone());
    assert!(response.is_success());

    if let ResponseData::Success { body, .. } = response {
      assert_eq!(body, large_data);
    } else {
      panic!("Expected Success variant");
    }
  }

  #[test]
  fn test_unicode_message() {
    let unicode_message = "错误消息 🚀";
    let response = ResponseData::error(StatusCode::BAD_REQUEST, unicode_message.to_string());
    assert!(response.is_error());

    if let ResponseData::Error { message, .. } = response {
      assert_eq!(message, unicode_message);
    } else {
      panic!("Expected Error variant");
    }
  }

  #[test]
  fn test_empty_message() {
    let response = ResponseData::error(StatusCode::BAD_REQUEST, String::new());
    assert!(response.is_error());
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
  }

  #[test]
  fn test_headers_with_multiple_values() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("cache-control", "no-cache".parse().unwrap());
    headers.insert("x-custom", "value".parse().unwrap());

    let response = ResponseData::success_with_headers(StatusCode::OK, Bytes::from("test"), headers);

    assert!(response.headers().contains_key("content-type"));
    assert!(response.headers().contains_key("cache-control"));
    assert!(response.headers().contains_key("x-custom"));
  }

  #[test]
  fn test_error_response_with_empty_headers() {
    let empty_headers = HeaderMap::new();
    let response =
      ResponseData::success_with_headers(StatusCode::OK, Bytes::from("test"), empty_headers);

    assert!(response.is_success());
    assert_eq!(response.headers().len(), 0);
  }
}
