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
  config: ConsumerConfig,
  buffer: String,
}

impl StringConsumer {
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
      buffer: String::new(),
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      config: ConsumerConfig::default(),
      buffer: String::with_capacity(capacity),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
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
          let strategy = self.handle_error(e.clone());
          match strategy {
            ErrorStrategy::Stop => return Err(e),
            ErrorStrategy::Skip => continue,
            ErrorStrategy::Retry(n) if e.retries < n => {
              // Retry logic would go here
              return Err(e);
            }
            _ => return Err(e),
          }
        }
      }
    }
    Ok(())
  }

  fn config(&self) -> &ConsumerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ConsumerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
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
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "string_consumer".to_string()),
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

    let config = consumer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_consumer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      consumer.create_error_context(None),
      consumer.component_info(),
    );

    assert_eq!(consumer.handle_error(error), ErrorStrategy::Skip);
  }
}
