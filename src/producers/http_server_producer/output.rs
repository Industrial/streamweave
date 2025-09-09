use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use crate::output::Output;
use std::pin::Pin;
use tokio_stream::Stream;

// Re-export types from their individual files
pub use super::http_request_channel::HttpRequestChannel;
pub use super::http_request_receiver::HttpRequestReceiver;
pub use super::http_server_producer_output::HttpServerProducerOutput;
pub use crate::http::request_stream::HttpRequestStream;

impl Output for super::http_server_producer::HttpServerProducer {
  type Output = StreamWeaveHttpRequestChunk;
  type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
}
