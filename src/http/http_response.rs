use bytes::Bytes;
use http::{HeaderMap, StatusCode};

/// Enhanced HTTP response representation for StreamWeave
#[derive(Debug, Clone)]
pub struct StreamWeaveHttpResponse {
  pub status: StatusCode,
  pub headers: HeaderMap,
  pub body: Bytes,
  pub trailers: Option<HeaderMap>,
}

impl StreamWeaveHttpResponse {
  pub fn new(status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
    Self {
      status,
      headers,
      body,
      trailers: None,
    }
  }

  pub fn ok(body: Bytes) -> Self {
    Self {
      status: StatusCode::OK,
      headers: HeaderMap::new(),
      body,
      trailers: None,
    }
  }

  pub fn not_found(body: Bytes) -> Self {
    Self {
      status: StatusCode::NOT_FOUND,
      headers: HeaderMap::new(),
      body,
      trailers: None,
    }
  }

  pub fn internal_server_error(body: Bytes) -> Self {
    Self {
      status: StatusCode::INTERNAL_SERVER_ERROR,
      headers: HeaderMap::new(),
      body,
      trailers: None,
    }
  }

  pub fn bad_request(body: Bytes) -> Self {
    Self {
      status: StatusCode::BAD_REQUEST,
      headers: HeaderMap::new(),
      body,
      trailers: None,
    }
  }

  pub fn unauthorized(body: Bytes) -> Self {
    Self {
      status: StatusCode::UNAUTHORIZED,
      headers: HeaderMap::new(),
      body,
      trailers: None,
    }
  }

  pub fn forbidden(body: Bytes) -> Self {
    Self {
      status: StatusCode::FORBIDDEN,
      headers: HeaderMap::new(),
      body,
      trailers: None,
    }
  }

  pub fn with_header(mut self, key: &str, value: &str) -> Self {
    if let Ok(key) = key.parse::<http::header::HeaderName>()
      && let Ok(value) = value.parse::<http::header::HeaderValue>()
    {
      self.headers.insert(key, value);
    }
    self
  }

  pub fn with_content_type(self, content_type: &str) -> Self {
    self.with_header("content-type", content_type)
  }

  pub fn with_trailers(mut self, trailers: HeaderMap) -> Self {
    self.trailers = Some(trailers);
    self
  }

  /// Convert to Axum Response for compatibility
  pub fn into_axum_response(self) -> axum::response::Response<axum::body::Body> {
    let mut response = axum::response::Response::builder()
      .status(self.status)
      .body(axum::body::Body::from(self.body))
      .unwrap();

    // Copy headers
    for (key, value) in self.headers {
      if let Some(key) = key {
        response.headers_mut().insert(key, value);
      }
    }

    response
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_creation() {
    let response = StreamWeaveHttpResponse::ok(Bytes::from("Hello, World!"));
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, Bytes::from("Hello, World!"));
    assert!(response.trailers.is_none());
  }

  #[test]
  fn test_response_with_headers() {
    let response = StreamWeaveHttpResponse::ok(Bytes::from("test"))
      .with_content_type("application/json")
      .with_header("x-custom", "value");

    assert!(response.headers.contains_key("content-type"));
    assert!(response.headers.contains_key("x-custom"));
  }
}
