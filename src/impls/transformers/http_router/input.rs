use crate::structs::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use crate::traits::input::Input;

/// Input trait implementation for HttpRouterTransformer
#[derive(Debug, Clone)]
pub struct HttpRouterInput;

impl Input for HttpRouterInput {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
}
