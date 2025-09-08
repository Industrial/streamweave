use crate::traits::producer::ProducerConfig;
use http::{HeaderMap, Method, Uri};

pub struct HttpRequestProducer {
  pub request: crate::structs::http::http_request::StreamWeaveHttpRequest,
  pub config: ProducerConfig<crate::structs::http::http_request::StreamWeaveHttpRequest>,
}

pub struct StreamingHttpRequestProducer {
  pub method: Method,
  pub uri: Uri,
  pub headers: HeaderMap,
  pub config: ProducerConfig<crate::structs::http::http_request_chunk::StreamWeaveHttpRequestChunk>,
}
