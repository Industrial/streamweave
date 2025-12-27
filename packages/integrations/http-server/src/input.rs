use crate::consumer::HttpResponseConsumer;
use crate::types::HttpResponse;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl Input for HttpResponseConsumer {
  type Input = HttpResponse;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::consumer::HttpResponseCorrelationConsumer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave_message::Message;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
impl Input for HttpResponseCorrelationConsumer {
  type Input = Message<HttpResponse>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
