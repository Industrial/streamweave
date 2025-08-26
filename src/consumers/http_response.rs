use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use axum::body::Body;
use axum::response::Response;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::{HeaderMap, StatusCode};
use std::pin::Pin;
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
    if let Ok(key) = key.parse::<http::header::HeaderName>() {
      if let Ok(value) = value.parse::<http::header::HeaderValue>() {
        self.headers.insert(key, value);
      }
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

pub struct HttpResponseConsumer {
  response_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<StreamWeaveHttpResponse>>>>,
  config: ConsumerConfig<StreamWeaveHttpResponse>,
}

impl HttpResponseConsumer {
  pub fn new() -> (
    Self,
    tokio::sync::oneshot::Receiver<StreamWeaveHttpResponse>,
  ) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let consumer = Self {
      response_sender: Arc::new(Mutex::new(Some(tx))),
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }
}

impl Input for HttpResponseConsumer {
  type Input = StreamWeaveHttpResponse;
  type InputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpResponse> + Send>>;
}

#[async_trait]
impl Consumer for HttpResponseConsumer {
  async fn consume(&mut self, mut stream: Self::InputStream) {
    if let Some(response) = stream.next().await {
      let mut sender = self.response_sender.lock().await;
      if let Some(tx) = sender.take() {
        let _ = tx.send(response);
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<Self::Input> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
