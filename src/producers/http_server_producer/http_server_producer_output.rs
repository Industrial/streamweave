use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use std::pin::Pin;
use tokio_stream::Stream;

/// Output type for HTTP Server Producer
pub type HttpServerProducerOutput = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
