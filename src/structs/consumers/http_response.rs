use crate::traits::consumer::ConsumerConfig;
use axum::body::Body;
use axum::response::Response;
use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct StreamWeaveHttpResponse {
  pub status: StatusCode,
  pub headers: HeaderMap,
  pub body: Bytes,
}

impl StreamWeaveHttpResponse {
  pub fn new(status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
    Self {
      status,
      headers,
      body,
    }
  }

  pub fn ok(body: Bytes) -> Self {
    Self {
      status: StatusCode::OK,
      headers: HeaderMap::new(),
      body,
    }
  }

  pub fn not_found(body: Bytes) -> Self {
    Self {
      status: StatusCode::NOT_FOUND,
      headers: HeaderMap::new(),
      body,
    }
  }

  pub fn internal_server_error(body: Bytes) -> Self {
    Self {
      status: StatusCode::INTERNAL_SERVER_ERROR,
      headers: HeaderMap::new(),
      body,
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

  pub fn into_axum_response(self) -> Response<Body> {
    let mut response = Response::builder()
      .status(self.status)
      .body(Body::from(self.body))
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

// New streaming response types
#[derive(Debug, Clone)]
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
}

pub struct HttpResponseConsumer {
  pub response_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<StreamWeaveHttpResponse>>>>,
  pub config: ConsumerConfig<StreamWeaveHttpResponse>,
}

// New streaming response consumer
pub struct StreamingHttpResponseConsumer {
  pub chunk_sender: tokio::sync::mpsc::Sender<ResponseChunk>,
  pub config: ConsumerConfig<ResponseChunk>,
}
