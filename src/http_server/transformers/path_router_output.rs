#[cfg(feature = "http-server")]
use crate::http_server::transformers::path_router::PathBasedRouterTransformer;
#[cfg(feature = "http-server")]
use crate::http_server::types::HttpRequest;
#[cfg(feature = "http-server")]
use crate::message::Message;
#[cfg(feature = "http-server")]
use crate::output::Output;
#[cfg(feature = "http-server")]
use futures::Stream;
#[cfg(feature = "http-server")]
use std::pin::Pin;

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
