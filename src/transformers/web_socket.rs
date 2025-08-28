use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;

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

pub struct WebSocketTransformer {
  config: TransformerConfig<WebSocketMessage>,
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

impl Clone for WebSocketTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for WebSocketTransformer {
  type Input = WebSocketMessage;
  type InputStream = Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>>;
}

impl Output for WebSocketTransformer {
  type Output = WebSocketMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>>;
}

// Helper function to create consistent stream types
fn create_message_stream(
  msg: WebSocketMessage,
) -> Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>> {
  match msg {
    WebSocketMessage::Text(text) => Box::pin(futures::stream::once(async move {
      WebSocketMessage::Text(format!("Echo: {}", text))
    })),
    WebSocketMessage::Binary(data) => Box::pin(futures::stream::once(async move {
      WebSocketMessage::Binary(data)
    })),
    WebSocketMessage::Ping => {
      Box::pin(futures::stream::once(async move { WebSocketMessage::Pong }))
    }
    WebSocketMessage::Pong => {
      Box::pin(futures::stream::once(async move { WebSocketMessage::Pong }))
    }
    WebSocketMessage::Close => Box::pin(futures::stream::once(
      async move { WebSocketMessage::Close },
    )),
  }
}

#[async_trait]
impl Transformer for WebSocketTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.flat_map(create_message_stream))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "web_socket_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio_stream::wrappers::ReceiverStream;

  #[tokio::test]
  async fn test_web_socket_transformer_text() {
    let mut transformer = WebSocketTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send text message
    tx.send(WebSocketMessage::Text("Hello".to_string()))
      .await
      .unwrap();
    drop(tx);

    // Check response
    if let Some(response) = output.next().await {
      match response {
        WebSocketMessage::Text(text) => {
          assert_eq!(text, "Echo: Hello");
        }
        _ => panic!("Expected text message"),
      }
    } else {
      panic!("Expected response");
    }
  }

  #[tokio::test]
  async fn test_web_socket_transformer_binary() {
    let mut transformer = WebSocketTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send binary message
    let data = Bytes::from("binary data");
    tx.send(WebSocketMessage::Binary(data.clone()))
      .await
      .unwrap();
    drop(tx);

    // Check response
    if let Some(response) = output.next().await {
      match response {
        WebSocketMessage::Binary(response_data) => {
          assert_eq!(response_data, data);
        }
        _ => panic!("Expected binary message"),
      }
    } else {
      panic!("Expected response");
    }
  }

  #[tokio::test]
  async fn test_web_socket_transformer_ping_pong() {
    let mut transformer = WebSocketTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send ping
    tx.send(WebSocketMessage::Ping).await.unwrap();
    drop(tx);

    // Check pong response
    if let Some(response) = output.next().await {
      match response {
        WebSocketMessage::Pong => {
          // Expected
        }
        _ => panic!("Expected pong message"),
      }
    } else {
      panic!("Expected response");
    }
  }

  #[test]
  fn test_web_socket_message_control_flags() {
    assert!(!WebSocketMessage::Text("hello".to_string()).is_control());
    assert!(!WebSocketMessage::Binary(Bytes::from("data")).is_control());
    assert!(WebSocketMessage::Ping.is_control());
    assert!(WebSocketMessage::Pong.is_control());
    assert!(WebSocketMessage::Close.is_control());
  }
}
