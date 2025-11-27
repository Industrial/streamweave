#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::producer::HttpRequestProducer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::types::HttpRequest;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::output::Output;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use futures::Stream;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::pin::Pin;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Output for HttpRequestProducer {
  type Output = HttpRequest;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
