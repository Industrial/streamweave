use super::vec_consumer::VecConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Consumer for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let consumer_name = self.config.name.clone();
    println!("📥 [{}] Starting to consume stream", consumer_name);
    let mut count = 0;
    while let Some(value) = stream.next().await {
      count += 1;
      println!(
        "   📦 [{}] Consuming item #{}: {:?}",
        consumer_name, count, value
      );
      self.vec.push(value);
    }
    println!("✅ [{}] Finished consuming {} items", consumer_name, count);
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
  async fn test_vec_consumer_basic() {
    let mut consumer = VecConsumer::new();
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let vec = consumer.into_vec();
    assert_eq!(vec, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_vec_consumer_empty_input() {
    let mut consumer = VecConsumer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let vec = consumer.into_vec();
    assert!(vec.is_empty());
  }

  #[tokio::test]
  async fn test_vec_consumer_with_capacity() {
    let mut consumer = VecConsumer::with_capacity(100);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(consumer.into_vec(), vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let consumer = VecConsumer::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
