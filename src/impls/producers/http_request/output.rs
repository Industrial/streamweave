use crate::structs::producers::http_request::{
  HttpRequestProducer, StreamWeaveHttpRequest, StreamWeaveHttpRequestChunk,
  StreamingHttpRequestProducer,
};
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
