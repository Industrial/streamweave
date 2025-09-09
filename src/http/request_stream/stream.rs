use super::http_request_stream::HttpRequestStream;
use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::Stream;

impl Stream for HttpRequestStream {
  type Item = StreamWeaveHttpRequestChunk;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.receiver.poll_recv(cx)
  }
}
