use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use async_trait::async_trait;
use axum::extract::Request;
use bytes::Bytes;
use futures::Stream;
use http::{HeaderMap, Method, Uri};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct StreamWeaveHttpRequest {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub body: Bytes,
}

impl StreamWeaveHttpRequest {
  pub fn new(method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Self {
    Self {
      method,
      uri,
      headers,
      body,
    }
  }

  pub fn from_axum_request(req: Request) -> impl std::future::Future<Output = Self> + Send {
    async move {
      let (parts, body) = req.into_parts();
      let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

      Self {
        method: parts.method,
        uri: parts.uri,
        headers: parts.headers,
        body: body_bytes,
      }
    }
  }
}

pub struct HttpRequestProducer {
  request: StreamWeaveHttpRequest,
  config: ProducerConfig<StreamWeaveHttpRequest>,
}

impl HttpRequestProducer {
  pub fn new(request: StreamWeaveHttpRequest) -> Self {
    Self {
      request,
      config: ProducerConfig::default(),
    }
  }

  pub fn from_axum_request(req: Request) -> impl std::future::Future<Output = Self> + Send {
    async move {
      let streamweave_req = StreamWeaveHttpRequest::from_axum_request(req).await;
      Self::new(streamweave_req)
    }
  }
}

impl Output for HttpRequestProducer {
  type Output = StreamWeaveHttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequest> + Send>>;
}

#[async_trait]
impl Producer for HttpRequestProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let request = self.request.clone();
    Box::pin(futures::stream::once(async move { request }))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Output>) -> ErrorContext<Self::Output> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "http_request_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "http_request_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
