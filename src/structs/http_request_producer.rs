use crate::traits::producer::ProducerConfig;
use bytes::Bytes;
use http::{HeaderMap, Method, Uri};

#[derive(Debug, Clone)]
pub struct StreamWeaveHttpRequest {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub body: Bytes,
}

#[derive(Debug, Clone)]
pub struct StreamWeaveHttpRequestChunk {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub chunk: Bytes,
}

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
