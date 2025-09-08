use crate::error::ErrorStrategy;
use crate::structs::consumers::http_response::{
  HttpResponseConsumer, StreamingHttpResponseConsumer,
};
use crate::traits::consumer::ConsumerConfig;
use std::sync::Arc;
use tokio::sync::Mutex;

impl HttpResponseConsumer {
  pub fn new() -> (
    Self,
    tokio::sync::oneshot::Receiver<
      crate::structs::consumers::http_response::StreamWeaveHttpResponse,
    >,
  ) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let consumer = Self {
      response_sender: Arc::new(Mutex::new(Some(tx))),
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }

  pub fn with_error_strategy(
    mut self,
    strategy: ErrorStrategy<crate::structs::consumers::http_response::StreamWeaveHttpResponse>,
  ) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl StreamingHttpResponseConsumer {
  pub fn new() -> (
    Self,
    tokio::sync::mpsc::Receiver<crate::structs::consumers::http_response::ResponseChunk>,
  ) {
    let (tx, rx) = tokio::sync::mpsc::channel(100); // Buffer size of 100 chunks
    let consumer = Self {
      chunk_sender: tx,
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }

  pub fn with_buffer_size(
    buffer_size: usize,
  ) -> (
    Self,
    tokio::sync::mpsc::Receiver<crate::structs::consumers::http_response::ResponseChunk>,
  ) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer_size);
    let consumer = Self {
      chunk_sender: tx,
      config: ConsumerConfig::default(),
    };
    (consumer, rx)
  }

  pub fn with_error_strategy(
    mut self,
    strategy: ErrorStrategy<crate::structs::consumers::http_response::ResponseChunk>,
  ) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
