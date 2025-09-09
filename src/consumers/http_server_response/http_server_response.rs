use crate::http::connection::RequestId;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};

/// HTTP response that can be sent back to a client
#[derive(Debug, Clone)]
pub struct HttpServerResponse {
  pub request_id: RequestId,
  pub status: StatusCode,
  pub headers: HeaderMap,
  pub body: Bytes,
}

impl HttpServerResponse {
  pub fn new(request_id: RequestId, status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
    Self {
      request_id,
      status,
      headers,
      body,
    }
  }

  pub fn ok(request_id: RequestId, body: Bytes) -> Self {
    Self::new(request_id, StatusCode::OK, HeaderMap::new(), body)
  }

  pub fn not_found(request_id: RequestId) -> Self {
    Self::new(
      request_id,
      StatusCode::NOT_FOUND,
      HeaderMap::new(),
      Bytes::from("Not Found"),
    )
  }

  pub fn internal_server_error(request_id: RequestId, message: &str) -> Self {
    Self::new(
      request_id,
      StatusCode::INTERNAL_SERVER_ERROR,
      HeaderMap::new(),
      Bytes::from(message.to_string()),
    )
  }

  pub fn with_header(mut self, name: &str, value: &str) -> Self {
    if let (Ok(name), Ok(value)) = (
      name.parse::<http::HeaderName>(),
      value.parse::<http::HeaderValue>(),
    ) {
      self.headers.insert(name, value);
    }
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use uuid::Uuid;

  #[test]
  fn test_http_server_response_creation() {
    let request_id = Uuid::new_v4();
    let response = HttpServerResponse::ok(request_id, Bytes::from("Hello, World!"));

    assert_eq!(response.request_id, request_id);
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, Bytes::from("Hello, World!"));
  }

  #[test]
  fn test_http_server_response_with_header() {
    let request_id = Uuid::new_v4();
    let response = HttpServerResponse::ok(request_id, Bytes::from("Test"))
      .with_header("Content-Type", "text/plain");

    assert!(response.headers.contains_key("content-type"));
  }

  #[test]
  fn test_not_found_response() {
    let request_id = Uuid::new_v4();
    let response = HttpServerResponse::not_found(request_id);

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.body, Bytes::from("Not Found"));
  }

  #[test]
  fn test_internal_server_error_response() {
    let request_id = Uuid::new_v4();
    let message = "Something went wrong";
    let response = HttpServerResponse::internal_server_error(request_id, message);

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.body, Bytes::from(message));
  }
}
