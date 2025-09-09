use crate::producers::http_server_producer::http_request_receiver::HttpRequestReceiver;

/// Stream wrapper for HTTP request chunks
pub struct HttpRequestStream {
  pub(crate) receiver: HttpRequestReceiver,
}

impl HttpRequestStream {
  pub fn new(receiver: HttpRequestReceiver) -> Self {
    Self { receiver }
  }
}
