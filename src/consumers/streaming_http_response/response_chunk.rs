use bytes::Bytes;
use http::{HeaderMap, StatusCode};

/// Streaming response chunk for processing
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseChunk {
  Header(StatusCode, HeaderMap),
  Body(Bytes),
  End,
  Error(StatusCode, String),
}

impl ResponseChunk {
  pub fn header(status: StatusCode, headers: HeaderMap) -> Self {
    Self::Header(status, headers)
  }

  pub fn body(data: Bytes) -> Self {
    Self::Body(data)
  }

  pub fn end() -> Self {
    Self::End
  }

  pub fn error(status: StatusCode, message: String) -> Self {
    Self::Error(status, message)
  }

  pub fn is_header(&self) -> bool {
    matches!(self, ResponseChunk::Header(_, _))
  }

  pub fn is_body(&self) -> bool {
    matches!(self, ResponseChunk::Body(_))
  }

  pub fn is_end(&self) -> bool {
    matches!(self, ResponseChunk::End)
  }

  pub fn is_error(&self) -> bool {
    matches!(self, ResponseChunk::Error(_, _))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_chunk() {
    let chunk = ResponseChunk::body(Bytes::from("data"));
    assert!(chunk.is_body());
    assert!(!chunk.is_end());

    let end_chunk = ResponseChunk::end();
    assert!(end_chunk.is_end());
    assert!(!end_chunk.is_body());
  }
}
