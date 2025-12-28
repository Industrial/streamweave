#[cfg(feature = "http-server")]
use crate::transformers::path_router::PathBasedRouterTransformer;
#[cfg(feature = "http-server")]
use crate::types::HttpRequest;
#[cfg(feature = "http-server")]
use futures::Stream;
#[cfg(feature = "http-server")]
use std::pin::Pin;
#[cfg(feature = "http-server")]
use streamweave::Output;
#[cfg(feature = "http-server")]
use streamweave_message::Message;

#[cfg(feature = "http-server")]
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
