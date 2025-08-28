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

      Self::new(parts.method, parts.uri, parts.headers, body_bytes)
    }
  }

  pub fn method(&self) -> &Method {
    &self.method
  }

  pub fn uri(&self) -> &Uri {
    &self.uri
  }

  pub fn headers(&self) -> &HeaderMap {
    &self.headers
  }
}

#[derive(Debug, Clone)]
pub struct StreamWeaveHttpRequestChunk {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub chunk: Bytes,
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

// New streaming producer for true streaming capabilities
pub struct StreamingHttpRequestProducer {
  method: Method,
  uri: Uri,
  headers: HeaderMap,
  config: ProducerConfig<StreamWeaveHttpRequestChunk>,
}

impl StreamingHttpRequestProducer {
  pub fn new(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    _body_stream: Pin<Box<dyn Stream<Item = Result<Bytes, axum::Error>> + Send>>,
  ) -> Self {
    Self {
      method,
      uri,
      headers,
      config: ProducerConfig::default(),
    }
  }

  pub fn from_axum_request(req: Request) -> impl std::future::Future<Output = Self> + Send {
    async move {
      let (parts, body) = req.into_parts();
      // Use a simpler approach - convert to bytes for now
      let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

      // Create a stream from the bytes
      let body_stream = Box::pin(futures::stream::once(async move { Ok(body_bytes) }));

      Self::new(parts.method, parts.uri, parts.headers, body_stream)
    }
  }
}

impl Output for HttpRequestProducer {
  type Output = StreamWeaveHttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequest> + Send>>;
}

impl Output for StreamingHttpRequestProducer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
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

#[async_trait]
impl Producer for StreamingHttpRequestProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let method = self.method.clone();
    let uri = self.uri.clone();
    let headers = self.headers.clone();

    // Create a new stream that processes the body stream
    Box::pin(futures::stream::once(async move {
      // For now, we'll just create a single chunk with the method/uri/headers
      // In a real implementation, this would process the actual body stream
      StreamWeaveHttpRequestChunk {
        method,
        uri,
        headers,
        chunk: Bytes::from("Streaming request body"),
      }
    }))
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
        .unwrap_or_else(|| "streaming_http_request_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "streaming_http_request_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
