#[cfg(feature = "http-server")]
use crate::producer::HttpRequestProducer;
#[cfg(feature = "http-server")]
use crate::types::HttpRequest;
#[cfg(feature = "http-server")]
use futures::Stream;
#[cfg(feature = "http-server")]
use std::pin::Pin;
#[cfg(feature = "http-server")]
use streamweave_core::Output;

#[cfg(feature = "http-server")]
impl Output for HttpRequestProducer {
  type Output = HttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[cfg(feature = "http-server")]
use crate::producer::LongLivedHttpRequestProducer;
#[cfg(feature = "http-server")]
use streamweave_message::Message;

#[cfg(feature = "http-server")]
impl Output for LongLivedHttpRequestProducer {
  type Output = Message<HttpRequest>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
