use crate::http_server::consumer::HttpResponseConsumer;
use crate::http_server::types::HttpResponse;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl Input for HttpResponseConsumer {
  type Input = HttpResponse;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
