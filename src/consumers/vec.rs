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

pub struct VecConsumer<T> {
  vec: Vec<T>,
  config: ConsumerConfig<T>,
}

impl<T> VecConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug,
{
  pub fn new() -> Self {
    Self {
      vec: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      vec: Vec::with_capacity(capacity),
      config: ConsumerConfig::default(),
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

  pub fn into_vec(self) -> Vec<T> {
    self.vec
  }
}

impl<T> crate::traits::error::Error for VecConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug,
{
  type Error = StreamError<T>;
}

impl<T> Input for VecConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError<T>>> + Send>>;
}

#[async_trait]
impl<T> Consumer for VecConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug,
{
  async fn consume(&mut self, input: Self::InputStream) -> Result<(), StreamError<T>> {
    let mut stream = input;
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          self.vec.push(value);
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

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.get_config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(_) => ErrorAction::Retry,
      ErrorStrategy::Custom(_) => ErrorAction::Skip,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer(self.component_info().name),
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
  async fn test_vec_consumer_basic() {
    let mut consumer = VecConsumer::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    let vec = consumer.into_vec();
    assert_eq!(vec, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_vec_consumer_empty_input() {
    let mut consumer = VecConsumer::new();
    let input = stream::iter(Vec::<Result<i32, StreamError<i32>>>::new());
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    let vec = consumer.into_vec();
    assert!(vec.is_empty());
  }

  #[tokio::test]
  async fn test_vec_consumer_with_capacity() {
    let mut consumer = VecConsumer::with_capacity(100);
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    assert_eq!(consumer.into_vec(), vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_vec_consumer_with_error() {
    let mut consumer = VecConsumer::new();
    let input = stream::iter(vec![
      Ok(1),
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
      Ok(2),
    ]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut consumer = VecConsumer::new()
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
