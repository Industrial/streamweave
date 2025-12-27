#[cfg(feature = "http-server")]
use crate::http_server::producer::HttpRequestProducer;
#[cfg(feature = "http-server")]
use crate::http_server::types::HttpRequest;
#[cfg(feature = "http-server")]
use crate::output::Output;
#[cfg(feature = "http-server")]
use futures::Stream;
#[cfg(feature = "http-server")]
use std::pin::Pin;

#[cfg(feature = "http-server")]
impl Output for HttpRequestProducer {
  type Output = HttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[cfg(feature = "http-server")]
use crate::http_server::producer::LongLivedHttpRequestProducer;
#[cfg(feature = "http-server")]
use crate::message::Message;

#[cfg(feature = "http-server")]
impl Output for LongLivedHttpRequestProducer {
  type Output = Message<HttpRequest>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
