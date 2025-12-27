#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::producer::HttpRequestProducer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::types::HttpRequest;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use futures::Stream;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::pin::Pin;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave_core::Output;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Output for HttpRequestProducer {
  type Output = HttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::producer::LongLivedHttpRequestProducer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave_message::Message;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Output for LongLivedHttpRequestProducer {
  type Output = Message<HttpRequest>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
