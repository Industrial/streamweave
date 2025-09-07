use crate::structs::http_request_producer::{
  HttpRequestProducer, StreamWeaveHttpRequest, StreamingHttpRequestProducer,
};
use crate::traits::producer::ProducerConfig;
use axum::extract::Request;
use bytes::Bytes;
use futures::Stream;
use http::{HeaderMap, Method, Uri};
use std::pin::Pin;

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
