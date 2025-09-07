use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::array_consumer::ArrayConsumer;
use crate::traits::consumer::{Consumer, ConsumerConfig};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T, const N: usize> Consumer for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
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
  use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use futures::stream;

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let consumer = ArrayConsumer::<i32, 3>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, "test_consumer");
  }

  #[tokio::test]
  async fn test_error_handling_during_consumption() {
    let consumer = ArrayConsumer::<i32, 3>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    // Test that Skip strategy allows consumption to continue
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(42),
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    });
    assert_eq!(action, ErrorAction::Skip);

    // Test that Stop strategy halts consumption
    let consumer = consumer.with_error_strategy(ErrorStrategy::<i32>::Stop);
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(42),
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    });
    assert_eq!(action, ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_component_info() {
    let consumer = ArrayConsumer::<i32, 3>::new().with_name("test_consumer".to_string());

    let info = consumer.component_info();
    assert_eq!(info.name, "test_consumer");
    assert_eq!(
      info.type_name,
      "streamweave::structs::array_consumer::ArrayConsumer<i32, 3>"
    );
  }

  #[tokio::test]
  async fn test_error_context_creation() {
    let consumer = ArrayConsumer::<i32, 3>::new().with_name("test_consumer".to_string());

    let context = consumer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_consumer");
    assert_eq!(
      context.component_type,
      "streamweave::structs::array_consumer::ArrayConsumer<i32, 3>"
    );
    assert_eq!(context.item, Some(42));
  }

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
}
