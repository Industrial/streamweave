use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use tokio::sync::mpsc;

/// Channel receiver for HTTP request chunks
pub type HttpRequestReceiver = mpsc::UnboundedReceiver<StreamWeaveHttpRequestChunk>;
