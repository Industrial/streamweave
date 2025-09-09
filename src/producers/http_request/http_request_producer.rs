use crate::http::http_request::StreamWeaveHttpRequest;
use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use crate::producer::ProducerConfig;
use axum::extract::Request;
use bytes::Bytes;
use futures::Stream;
use http::{HeaderMap, Method, Uri};
use std::pin::Pin;

pub struct HttpRequestProducer {
  pub request: StreamWeaveHttpRequest,
  pub config: ProducerConfig<StreamWeaveHttpRequest>,
}

pub struct StreamingHttpRequestProducer {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub config: ProducerConfig<StreamWeaveHttpRequestChunk>,
}

impl HttpRequestProducer {
  pub fn new(request: StreamWeaveHttpRequest) -> Self {
    Self {
      request,
      config: ProducerConfig::default(),
    }
  }

  pub async fn from_axum_request(req: Request) -> Self {
    let streamweave_req = StreamWeaveHttpRequest::from_axum_request(req).await;
    Self::new(streamweave_req)
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

  pub async fn from_axum_request(req: Request) -> Self {
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
