use crate::http::{
  http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
};

/// Trait for handling HTTP requests
#[async_trait::async_trait]
pub trait HttpHandler: Send + Sync {
  /// Handle an HTTP request and return a response
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse;
}
