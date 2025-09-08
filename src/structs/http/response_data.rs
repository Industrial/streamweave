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
