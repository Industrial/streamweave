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

pub struct ArrayConsumer<T, const N: usize> {
  config: ConsumerConfig,
  array: [Option<T>; N],
  index: usize,
}

impl<T, const N: usize> ArrayConsumer<T, N>
where
  T: Send + 'static,
{
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
      array: std::array::from_fn(|_| None),
      index: 0,
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
              self.create_error_context(None),
              self.component_info(),
            ));
          }
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
        .unwrap_or_else(|| "array_consumer".to_string()),
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
    let mut consumer = ArrayConsumer::<i32, 5>::new();
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    let array = consumer.into_array();
    assert_eq!(array[0], Some(1));
    assert_eq!(array[1], Some(2));
    assert_eq!(array[2], Some(3));
    assert_eq!(array[3], None);
    assert_eq!(array[4], None);
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
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
    if let Err(e) = result {
      assert!(e.to_string().contains("Array capacity exceeded"));
    }
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
