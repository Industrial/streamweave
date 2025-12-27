#[cfg(feature = "http-server")]
use crate::transformers::path_router::PathBasedRouterTransformer;
#[cfg(feature = "http-server")]
use crate::types::HttpRequest;
#[cfg(feature = "http-server")]
use futures::Stream;
#[cfg(feature = "http-server")]
use std::pin::Pin;
#[cfg(feature = "http-server")]
use streamweave::Input;
#[cfg(feature = "http-server")]
use streamweave_message::Message;

#[cfg(feature = "http-server")]
impl Input for PathBasedRouterTransformer {
  type Input = Message<HttpRequest>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
