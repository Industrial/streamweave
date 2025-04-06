use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct StringConsumer {
  buffer: String,
}

impl StringConsumer {
  pub fn new() -> Self {
    Self {
      buffer: String::new(),
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      buffer: String::with_capacity(capacity),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    let mut config = self.get_config();
    config.error_strategy = strategy;
    self.set_config(config);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    let mut config = self.get_config();
    config.name = name;
    self.set_config(config);
    self
  }

  pub fn into_string(self) -> String {
    self.buffer
  }
}

impl crate::traits::error::Error for StringConsumer {
  type Error = StreamError;
}

impl Input for StringConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

#[async_trait]
impl Consumer for StringConsumer {
  async fn consume(&mut self, input: Self::InputStream) -> Result<(), StreamError> {
    let mut stream = input;
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          self.buffer.push_str(&value);
        }
        Err(e) => {
          let action = self.handle_error(&e);
          match action {
            ErrorAction::Stop => return Err(e),
            ErrorAction::Skip => continue,
            ErrorAction::Retry => {
              // Retry logic would go here
              return Err(e);
            }
          }
        }
      }
    }
    Ok(())
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.get_config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name,
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
        .map(|s| Ok(s.to_string())),
    );
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    assert_eq!(consumer.into_string(), "hello world");
  }

  #[tokio::test]
  async fn test_string_consumer_empty_input() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(Vec::<Result<String, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    assert!(consumer.into_string().is_empty());
  }

  #[tokio::test]
  async fn test_string_consumer_with_capacity() {
    let mut consumer = StringConsumer::with_capacity(100);
    let input = stream::iter(
      vec!["hello", " ", "world"]
        .into_iter()
        .map(|s| Ok(s.to_string())),
    );
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    assert_eq!(consumer.into_string(), "hello world");
  }

  #[tokio::test]
  async fn test_string_consumer_with_error() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(vec![
      Ok("hello".to_string()),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Consumer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok("world".to_string()),
    ]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut consumer = StringConsumer::new()
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::Skip);
    assert_eq!(config.name, "test_consumer");

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      consumer.create_error_context(None),
      consumer.component_info(),
    );

    assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
  }
}
