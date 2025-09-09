use crate::http::{
  http_request::StreamWeaveHttpRequest, http_request_chunk::StreamWeaveHttpRequestChunk,
};
use super::http_request_producer::{HttpRequestProducer, StreamingHttpRequestProducer};
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl Output for HttpRequestProducer {
  type Output = StreamWeaveHttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequest> + Send>>;
}

impl Output for StreamingHttpRequestProducer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
}
