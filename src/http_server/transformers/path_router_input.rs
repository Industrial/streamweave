#[cfg(feature = "http-server")]
use crate::http_server::transformers::path_router::PathBasedRouterTransformer;
#[cfg(feature = "http-server")]
use crate::http_server::types::HttpRequest;
#[cfg(feature = "http-server")]
use crate::input::Input;
#[cfg(feature = "http-server")]
use crate::message::Message;
#[cfg(feature = "http-server")]
use futures::Stream;
#[cfg(feature = "http-server")]
use std::pin::Pin;

#[cfg(feature = "http-server")]
impl Input for PathBasedRouterTransformer {
  type Input = Message<HttpRequest>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
