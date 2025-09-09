use crate::http::http_response::StreamWeaveHttpResponse;
use crate::output::Output;

/// Output trait implementation for HttpRouterTransformer
#[derive(Debug, Clone)]
pub struct HttpRouterOutput;

impl Output for HttpRouterOutput {
  type Output = StreamWeaveHttpResponse;
  type OutputStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = StreamWeaveHttpResponse> + Send>>;
}
