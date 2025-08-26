use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct StringConsumer {
  buffer: String,
  config: ConsumerConfig<String>,
}

impl StringConsumer {
  pub fn new() -> Self {
    Self {
      buffer: String::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      buffer: String::with_capacity(capacity),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  pub fn into_string(self) -> String {
    self.buffer
  }
}

impl Input for StringConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for StringConsumer {
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      self.buffer.push_str(&value);
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<String> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_string_consumer_basic() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(
      vec!["hello", " ", "world"]
        .into_iter()
        .map(|s| s.to_string()),
    );
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(consumer.into_string(), "hello world");
  }

  #[tokio::test]
  async fn test_string_consumer_empty_input() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(Vec::<String>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert!(consumer.into_string().is_empty());
  }

  #[tokio::test]
  async fn test_string_consumer_with_capacity() {
    let mut consumer = StringConsumer::with_capacity(100);
    let input = stream::iter(
      vec!["hello", " ", "world"]
        .into_iter()
        .map(|s| s.to_string()),
    );
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(consumer.into_string(), "hello world");
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut consumer = StringConsumer::new()
      .with_error_strategy(ErrorStrategy::<String>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<String>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
