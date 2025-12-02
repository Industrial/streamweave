use super::array_producer::ArrayProducer;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producer::{Producer, ProducerConfig};
use async_trait::async_trait;

#[async_trait]
impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> Producer
  for ArrayProducer<T, N>
{
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    let array = self.array.clone();
    Box::pin(futures::stream::iter(array))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "array_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "array_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[derive(Debug)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_array_producer_basic() {
    let mut producer = ArrayProducer::new([1, 2, 3, 4, 5]);
    let result: Vec<i32> = producer.produce().collect().await;
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_array_producer_empty() {
    let mut producer = ArrayProducer::<i32, 0>::new([]);
    let result: Vec<i32> = producer.produce().collect().await;
    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_array_producer_strings() {
    let mut producer = ArrayProducer::new(["a", "b", "c"]);
    let result: Vec<&str> = producer.produce().collect().await;
    assert_eq!(result, vec!["a", "b", "c"]);
  }

  #[tokio::test]
  async fn test_array_producer_with_error() {
    let producer = ArrayProducer::new([1, 2, 3]);
    let error = StreamError::new(
      Box::new(TestError("test error".to_string())),
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(1),
        component_name: "array_producer".to_string(),
        component_type: std::any::type_name::<ArrayProducer<i32, 3>>().to_string(),
      },
      ComponentInfo {
        name: "array_producer".to_string(),
        type_name: std::any::type_name::<ArrayProducer<i32, 3>>().to_string(),
      },
    );

    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = ArrayProducer::new([1, 2, 3])
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));
  }

  #[tokio::test]
  async fn test_array_producer_from_slice_success() {
    let slice = &[1, 2, 3];
    let mut producer = ArrayProducer::<_, 3>::from_slice(slice).unwrap();
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_producer_from_slice_failure() {
    let slice = &[1, 2, 3];
    assert!(ArrayProducer::<_, 4>::from_slice(slice).is_none());
  }

  #[tokio::test]
  async fn test_array_producer_multiple_calls() {
    let mut producer = ArrayProducer::new([1, 2, 3]);

    // First call
    let stream = producer.produce();
    let result1: Vec<i32> = stream.collect().await;
    assert_eq!(result1, vec![1, 2, 3]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<i32> = stream.collect().await;
    assert_eq!(result2, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_len_and_is_empty() {
    let producer: ArrayProducer<i32, 3> = ArrayProducer::new([1, 2, 3]);
    assert_eq!(producer.len(), 3);
    assert!(!producer.is_empty());

    let empty_producer: ArrayProducer<i32, 0> = ArrayProducer::new([]);
    assert_eq!(empty_producer.len(), 0);
    assert!(empty_producer.is_empty());
  }

  #[test]
  fn test_error_handling_stop() {
    let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "ArrayProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ArrayProducer".to_string(),
      },
      retries: 0,
    };

    let strategy = producer.handle_error(&error);
    assert!(matches!(strategy, ErrorAction::Stop));
  }

  #[test]
  fn test_error_handling_skip() {
    let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
    producer = producer.with_error_strategy(ErrorStrategy::Skip);
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "ArrayProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ArrayProducer".to_string(),
      },
      retries: 0,
    };

    let strategy = producer.handle_error(&error);
    assert!(matches!(strategy, ErrorAction::Skip));
  }
}
