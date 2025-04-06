use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{consumer::Consumer, input::Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct ArrayConsumer<T, const N: usize> {
  array: [Option<T>; N],
  index: usize,
}

impl<T, const N: usize> ArrayConsumer<T, N>
where
  T: Send + 'static,
{
  pub fn new() -> Self {
    Self {
      array: std::array::from_fn(|_| None),
      index: 0,
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

  pub fn into_array(self) -> [Option<T>; N] {
    self.array
  }
}

impl<T, const N: usize> crate::traits::error::Error for ArrayConsumer<T, N>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T, const N: usize> Input for ArrayConsumer<T, N>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

#[async_trait]
impl<T, const N: usize> Consumer for ArrayConsumer<T, N>
where
  T: Send + 'static,
{
  async fn consume(&mut self, input: Self::InputStream) -> Result<(), StreamError> {
    let mut stream = input;
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          if self.index < N {
            self.array[self.index] = Some(value);
            self.index += 1;
          } else {
            return Err(StreamError::new(
              Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Array capacity exceeded",
              )),
              self.create_error_context(Some(Box::new(value))),
              self.component_info(),
            ));
          }
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
  async fn test_array_consumer_basic() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    let array = consumer.into_array();
    assert_eq!(array[0], Some(1));
    assert_eq!(array[1], Some(2));
    assert_eq!(array[2], Some(3));
  }

  #[tokio::test]
  async fn test_array_consumer_empty_input() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    let array = consumer.into_array();
    assert_eq!(array[0], None);
    assert_eq!(array[1], None);
    assert_eq!(array[2], None);
  }

  #[tokio::test]
  async fn test_array_consumer_capacity_exceeded() {
    let mut consumer = ArrayConsumer::<i32, 2>::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_array_consumer_with_error() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
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
    let mut consumer = ArrayConsumer::<i32, 3>::new()
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
