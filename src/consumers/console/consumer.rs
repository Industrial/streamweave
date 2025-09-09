use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use super::console_consumer::ConsoleConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Consumer for ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      println!("{}", value);
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
  async fn test_console_consumer_integers() {
    let mut consumer = ConsoleConsumer::<i32>::new();
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_console_consumer_strings() {
    let mut consumer = ConsoleConsumer::<String>::new();
    let input = stream::iter(vec!["hello".to_string(), "world".to_string()]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_console_consumer_floats() {
    let mut consumer = ConsoleConsumer::<f64>::new();
    let input = stream::iter(vec![1.1, 2.2, 3.3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_console_consumer_custom_type() {
    #[derive(Debug, Clone)]
    struct CustomType(i32);

    impl std::fmt::Display for CustomType {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Custom({})", self.0)
      }
    }

    let mut consumer = ConsoleConsumer::<CustomType>::new();
    let input = stream::iter(vec![CustomType(1), CustomType(2), CustomType(3)]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_console_consumer_reuse() {
    let mut consumer = ConsoleConsumer::<i32>::new();

    // First consumption
    let input1 = stream::iter(vec![1, 2, 3]);
    let boxed_input1 = Box::pin(input1);
    consumer.consume(boxed_input1).await;

    // Second consumption - should work fine
    let input2 = stream::iter(vec![4, 5, 6]);
    let boxed_input2 = Box::pin(input2);
    consumer.consume(boxed_input2).await;
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let consumer = ConsoleConsumer::<i32>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
