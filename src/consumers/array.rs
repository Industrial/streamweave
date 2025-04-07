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
  array: [Option<T>; N],
  index: usize,
  config: ConsumerConfig<T>,
}

impl<T, const N: usize> ArrayConsumer<T, N>
where
  T: Send + Clone + 'static,
{
  pub fn new() -> Self {
    Self {
      array: std::array::from_fn(|_| None),
      index: 0,
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  pub fn into_array(self) -> [Option<T>; N] {
    self.array
  }
}

impl<T, const N: usize> Input for ArrayConsumer<T, N>
where
  T: Send + Clone + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T, const N: usize> Consumer for ArrayConsumer<T, N>
where
  T: Send + Clone + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      if self.index < N {
        self.array[self.index] = Some(value);
        self.index += 1;
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<T> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
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
  async fn test_array_consumer_basic() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    assert_eq!(array[0], Some(1));
    assert_eq!(array[1], Some(2));
    assert_eq!(array[2], Some(3));
  }

  #[tokio::test]
  async fn test_array_consumer_empty_input() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    assert_eq!(array[0], None);
    assert_eq!(array[1], None);
    assert_eq!(array[2], None);
  }

  #[tokio::test]
  async fn test_array_consumer_capacity_exceeded() {
    let mut consumer = ArrayConsumer::<i32, 2>::new();
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    assert_eq!(array[0], Some(1));
    assert_eq!(array[1], Some(2));
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut consumer = ArrayConsumer::<i32, 3>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
