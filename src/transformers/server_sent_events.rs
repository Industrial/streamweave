use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct SSEMessage {
  pub event: Option<String>,
  pub data: String,
  pub id: Option<String>,
  pub retry: Option<u64>,
}

impl SSEMessage {
  pub fn new(data: String) -> Self {
    Self {
      event: None,
      data,
      id: None,
      retry: None,
    }
  }

  pub fn with_event(mut self, event: String) -> Self {
    self.event = Some(event);
    self
  }

  pub fn with_id(mut self, id: String) -> Self {
    self.id = Some(id);
    self
  }

  pub fn with_retry(mut self, retry: u64) -> Self {
    self.retry = Some(retry);
    self
  }

  pub fn to_sse_format(&self) -> String {
    let mut sse_text = String::new();

    if let Some(event) = &self.event {
      sse_text.push_str(&format!("event: {}\n", event));
    }
    if let Some(id) = &self.id {
      sse_text.push_str(&format!("id: {}\n", id));
    }
    if let Some(retry) = self.retry {
      sse_text.push_str(&format!("retry: {}\n", retry));
    }

    sse_text.push_str(&format!("data: {}\n\n", self.data));
    sse_text
  }
}

pub struct ServerSentEventsTransformer {
  config: TransformerConfig<SSEMessage>,
}

impl ServerSentEventsTransformer {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<SSEMessage>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }
}

impl Clone for ServerSentEventsTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ServerSentEventsTransformer {
  type Input = SSEMessage;
  type InputStream = Pin<Box<dyn Stream<Item = SSEMessage> + Send>>;
}

impl Output for ServerSentEventsTransformer {
  type Output = SSEMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = SSEMessage> + Send>>;
}

#[async_trait]
impl Transformer for ServerSentEventsTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|msg| {
      // Transform the message to SSE format
      msg
    }))
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
        .unwrap_or_else(|| "server_sent_events_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio_stream::wrappers::ReceiverStream;

  #[test]
  fn test_sse_message_creation() {
    let message = SSEMessage::new("Hello World".to_string());
    assert_eq!(message.data, "Hello World");
    assert!(message.event.is_none());
    assert!(message.id.is_none());
    assert!(message.retry.is_none());
  }

  #[test]
  fn test_sse_message_with_event() {
    let message = SSEMessage::new("Hello World".to_string()).with_event("message".to_string());
    assert_eq!(message.event, Some("message".to_string()));
  }

  #[test]
  fn test_sse_message_with_id() {
    let message = SSEMessage::new("Hello World".to_string()).with_id("123".to_string());
    assert_eq!(message.id, Some("123".to_string()));
  }

  #[test]
  fn test_sse_message_with_retry() {
    let message = SSEMessage::new("Hello World".to_string()).with_retry(5000);
    assert_eq!(message.retry, Some(5000));
  }

  #[test]
  fn test_sse_message_format() {
    let message = SSEMessage::new("Hello World".to_string())
      .with_event("message".to_string())
      .with_id("123".to_string())
      .with_retry(5000);

    let formatted = message.to_sse_format();
    assert!(formatted.contains("event: message"));
    assert!(formatted.contains("id: 123"));
    assert!(formatted.contains("retry: 5000"));
    assert!(formatted.contains("data: Hello World"));
    assert!(formatted.ends_with("\n\n"));
  }

  #[tokio::test]
  async fn test_sse_transformer() {
    let mut transformer = ServerSentEventsTransformer::new();
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send SSE message
    let message = SSEMessage::new("Hello World".to_string()).with_event("message".to_string());
    tx.send(message.clone()).await.unwrap();
    drop(tx);

    // Check response
    if let Some(response) = output.next().await {
      assert_eq!(response.data, message.data);
      assert_eq!(response.event, message.event);
    } else {
      panic!("Expected response");
    }
  }
}
