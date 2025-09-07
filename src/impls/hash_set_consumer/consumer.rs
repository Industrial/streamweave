use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::hash_set_consumer::HashSetConsumer;
use crate::traits::consumer::{Consumer, ConsumerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use std::hash::Hash;

#[async_trait]
impl<T> Consumer for HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      self.set.insert(value);
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
  use crate::error::ErrorStrategy;
  use futures::stream;

  #[tokio::test]
  async fn test_hash_set_consumer_basic() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(vec![1, 2, 3, 1]); // Duplicate 1
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let set = consumer.into_set();
    assert_eq!(set.len(), 3); // Should have 3 unique elements
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
  }

  #[tokio::test]
  async fn test_hash_set_consumer_empty_input() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert!(consumer.into_set().is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let consumer = HashSetConsumer::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
