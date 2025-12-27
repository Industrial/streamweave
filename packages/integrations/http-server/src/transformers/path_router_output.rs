#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::transformers::path_router::PathBasedRouterTransformer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::types::HttpRequest;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use futures::Stream;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::pin::Pin;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave_core::Output;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave_message::Message;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Output for PathBasedRouterTransformer {
  type Output = (
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
  );
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
