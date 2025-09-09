use crate::http::http_request_chunk::StreamWeaveHttpRequestChunk;
use tokio::sync::mpsc;

/// Internal channel for HTTP request chunks
pub type HttpRequestChannel = mpsc::UnboundedSender<StreamWeaveHttpRequestChunk>;
