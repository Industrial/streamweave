use crate::traits::transformer::TransformerConfig;
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
