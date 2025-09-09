use bytes::Bytes;
use http::{HeaderMap, StatusCode};

/// Response data for transformer processing
#[derive(Debug, Clone)]
pub enum ResponseData {
  Success {
    status: StatusCode,
    body: Bytes,
    headers: HeaderMap,
  },
  Error {
    status: StatusCode,
    message: String,
  },
  Stream {
    status: StatusCode,
    headers: HeaderMap,
    stream_id: String,
  },
}

impl ResponseData {
  pub fn success(status: StatusCode, body: Bytes, headers: HeaderMap) -> Self {
    Self::Success {
      status,
      body,
      headers,
    }
  }

  pub fn error(status: StatusCode, message: String) -> Self {
    Self::Error { status, message }
  }

  pub fn stream(status: StatusCode, headers: HeaderMap, stream_id: String) -> Self {
    Self::Stream {
      status,
      headers,
      stream_id,
    }
  }

  pub fn status(&self) -> StatusCode {
    match self {
      ResponseData::Success { status, .. } => *status,
      ResponseData::Error { status, .. } => *status,
      ResponseData::Stream { status, .. } => *status,
    }
  }

  pub fn headers(&self) -> HeaderMap {
    match self {
      ResponseData::Success { headers, .. } => headers.clone(),
      ResponseData::Error { .. } => HeaderMap::new(),
      ResponseData::Stream { headers, .. } => headers.clone(),
    }
  }

  pub fn success_with_headers(status: StatusCode, body: Bytes, headers: HeaderMap) -> Self {
    Self::Success {
      status,
      body,
      headers,
    }
  }

  // Convenience methods for common status codes
  pub fn ok(body: Bytes) -> Self {
    Self::Success {
      status: StatusCode::OK,
      body,
      headers: HeaderMap::new(),
    }
  }

  pub fn not_found(message: String) -> Self {
    Self::Error {
      status: StatusCode::NOT_FOUND,
      message,
    }
  }

  pub fn internal_server_error(message: String) -> Self {
    Self::Error {
      status: StatusCode::INTERNAL_SERVER_ERROR,
      message,
    }
  }

  pub fn bad_request(message: String) -> Self {
    Self::Error {
      status: StatusCode::BAD_REQUEST,
      message,
    }
  }

  pub fn unauthorized(message: String) -> Self {
    Self::Error {
      status: StatusCode::UNAUTHORIZED,
      message,
    }
  }

  pub fn forbidden(message: String) -> Self {
    Self::Error {
      status: StatusCode::FORBIDDEN,
      message,
    }
  }

  pub fn body(&self) -> Bytes {
    match self {
      ResponseData::Success { body, .. } => body.clone(),
      ResponseData::Error { .. } => Bytes::new(),
      ResponseData::Stream { .. } => Bytes::new(),
    }
  }

  pub fn is_success(&self) -> bool {
    matches!(self, ResponseData::Success { .. })
  }

  pub fn is_error(&self) -> bool {
    matches!(self, ResponseData::Error { .. })
  }

  pub fn is_stream(&self) -> bool {
    matches!(self, ResponseData::Stream { .. })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_data() {
    let success = ResponseData::success(StatusCode::OK, Bytes::from("success"), HeaderMap::new());
    assert!(success.is_success());
    assert!(!success.is_error());

    let error = ResponseData::error(StatusCode::NOT_FOUND, "Not found".to_string());
    assert!(error.is_error());
    assert!(!error.is_success());
  }
}
