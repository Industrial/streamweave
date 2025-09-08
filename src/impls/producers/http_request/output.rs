use crate::structs::http::{
  http_request::StreamWeaveHttpRequest, http_request_chunk::StreamWeaveHttpRequestChunk,
};
use crate::structs::producers::http_request::{HttpRequestProducer, StreamingHttpRequestProducer};
use crate::traits::output::Output;
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
