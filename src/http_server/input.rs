use crate::http_server::consumer::HttpResponseConsumer;
use crate::http_server::types::HttpResponse;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for HttpResponseConsumer {
  type Input = HttpResponse;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::consumer::HttpResponseCorrelationConsumer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::message::Message;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Input for HttpResponseCorrelationConsumer {
  type Input = Message<HttpResponse>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
