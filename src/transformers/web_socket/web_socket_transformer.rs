use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub enum WebSocketMessage {
  Text(String),
  Binary(Bytes),
  Ping,
  Pong,
  Close,
}

impl WebSocketMessage {
  pub fn text(text: String) -> Self {
    Self::Text(text)
  }

  pub fn binary(data: Bytes) -> Self {
    Self::Binary(data)
  }

  pub fn ping() -> Self {
    Self::Ping
  }

  pub fn pong() -> Self {
    Self::Pong
  }

  pub fn close() -> Self {
    Self::Close
  }

  pub fn is_control(&self) -> bool {
    matches!(self, Self::Ping | Self::Pong | Self::Close)
  }
}

#[derive(Clone)]
pub struct WebSocketTransformer {
  pub config: TransformerConfig<WebSocketMessage>,
}

impl Default for WebSocketTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl WebSocketTransformer {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<WebSocketMessage>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }
}
