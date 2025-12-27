use crate::circuit_breaker_transformer::CircuitBreakerTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::atomic::Ordering;
use streamweave_core::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T> Transformer for CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let failure_count = self.failure_count.clone();
    let last_failure_time = self.last_failure_time.clone();
    let _failure_threshold = self.failure_threshold;
    let _reset_timeout = self.reset_timeout;

    Box::pin(
      input
        .map(move |item| {
          let failure_count = failure_count.clone();
          let _last_failure_time = last_failure_time.clone();
          async move {
            failure_count.store(0, Ordering::SeqCst);
            item
          }
        })
        .buffered(1),
    )
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
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
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "circuit_breaker_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;
  use proptest::prelude::*;
  use std::sync::Arc;
  use tokio::time::{Duration, sleep};

  #[tokio::test]
  async fn test_circuit_breaker_basic() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_circuit_breaker_empty_input() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  #[test]
  fn test_circuit_breaker_new() {
    let transformer = CircuitBreakerTransformer::<i32>::new(5, Duration::from_millis(200));

    assert_eq!(transformer.failure_threshold, 5);
    assert_eq!(transformer.reset_timeout, Duration::from_millis(200));
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 0);
    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_circuit_breaker_with_error_strategy() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::Skip);

    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_circuit_breaker_with_name() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100))
      .with_name("test_circuit_breaker".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("test_circuit_breaker".to_string())
    );
  }

  #[tokio::test]
  async fn test_circuit_breaker_is_circuit_open() {
    let transformer = CircuitBreakerTransformer::<i32>::new(2, Duration::from_millis(100));

    // Initially circuit should be closed
    assert!(!transformer._is_circuit_open().await);

    // Record one failure
    transformer._record_failure().await;
    assert!(!transformer._is_circuit_open().await);

    // Record second failure (threshold reached)
    transformer._record_failure().await;
    assert!(transformer._is_circuit_open().await);

    // Wait for reset timeout
    sleep(Duration::from_millis(150)).await;
    assert!(!transformer._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_record_failure() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100));

    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 0);

    transformer._record_failure().await;
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 1);

    transformer._record_failure().await;
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 2);

    transformer._record_failure().await;
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 3);
  }

  #[tokio::test]
  async fn test_circuit_breaker_failure_threshold_edge_cases() {
    // Test with threshold 0
    let transformer = CircuitBreakerTransformer::<i32>::new(0, Duration::from_millis(100));
    assert!(transformer._is_circuit_open().await);

    // Test with threshold 1
    let transformer = CircuitBreakerTransformer::<i32>::new(1, Duration::from_millis(100));
    assert!(!transformer._is_circuit_open().await);

    transformer._record_failure().await;
    assert!(transformer._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_reset_timeout() {
    let transformer = CircuitBreakerTransformer::<i32>::new(1, Duration::from_millis(50));

    // Record failure to open circuit
    transformer._record_failure().await;
    assert!(transformer._is_circuit_open().await);

    // Wait less than reset timeout
    sleep(Duration::from_millis(25)).await;
    assert!(transformer._is_circuit_open().await);

    // Wait for reset timeout
    sleep(Duration::from_millis(50)).await;
    assert!(!transformer._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_multiple_failures() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100));

    // Record multiple failures
    for i in 0..5 {
      transformer._record_failure().await;
      assert_eq!(transformer.failure_count.load(Ordering::SeqCst), i + 1);
    }

    // Circuit should be open
    assert!(transformer._is_circuit_open().await);

    // Wait for reset
    sleep(Duration::from_millis(150)).await;
    assert!(!transformer._is_circuit_open().await);
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 0);
  }

  #[tokio::test]
  async fn test_circuit_breaker_error_handling() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100));

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "CircuitBreakerTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CircuitBreakerTransformer".to_string(),
      },
      retries: 0,
    };

    // Test default error strategy (Stop)
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));

    // Test Skip strategy
    let transformer = transformer.with_error_strategy(ErrorStrategy::Skip);
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));

    // Test Retry strategy
    let transformer = transformer.with_error_strategy(ErrorStrategy::Retry(3));
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));

    // Test Retry strategy exhausted
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "CircuitBreakerTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CircuitBreakerTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_circuit_breaker_error_context_creation() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100))
      .with_name("test_circuit_breaker".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_circuit_breaker");
    assert_eq!(
      context.component_type,
      std::any::type_name::<CircuitBreakerTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_circuit_breaker_component_info() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100))
      .with_name("test_circuit_breaker".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_circuit_breaker");
    assert_eq!(
      info.type_name,
      std::any::type_name::<CircuitBreakerTransformer<i32>>()
    );
  }

  #[test]
  fn test_circuit_breaker_default_name() {
    let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100));

    let info = transformer.component_info();
    assert_eq!(info.name, "circuit_breaker_transformer");
  }

  #[test]
  fn test_circuit_breaker_config_mut() {
    let mut transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100));
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_circuit_breaker_concurrent_access() {
    let transformer = Arc::new(CircuitBreakerTransformer::<i32>::new(
      2,
      Duration::from_millis(100),
    ));

    // Spawn multiple tasks to record failures concurrently
    let mut handles = vec![];
    for _ in 0..5 {
      let transformer = Arc::clone(&transformer);
      handles.push(tokio::spawn(async move {
        transformer._record_failure().await;
      }));
    }

    // Wait for all tasks to complete
    for handle in handles {
      handle.await.unwrap();
    }

    // Check final failure count
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 5);
    assert!(transformer._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_different_data_types() {
    // Test with u64
    let transformer_u64 = CircuitBreakerTransformer::<u64>::new(2, Duration::from_millis(100));
    assert!(!transformer_u64._is_circuit_open().await);

    // Test with String
    let transformer_string =
      CircuitBreakerTransformer::<String>::new(2, Duration::from_millis(100));
    assert!(!transformer_string._is_circuit_open().await);

    // Test with f64
    let transformer_f64 = CircuitBreakerTransformer::<f64>::new(2, Duration::from_millis(100));
    assert!(!transformer_f64._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_reset_after_multiple_failures() {
    let transformer = CircuitBreakerTransformer::<i32>::new(2, Duration::from_millis(50));

    // Record failures to open circuit
    transformer._record_failure().await;
    transformer._record_failure().await;
    assert!(transformer._is_circuit_open().await);

    // Wait for reset
    sleep(Duration::from_millis(100)).await;
    assert!(!transformer._is_circuit_open().await);
    assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 0);

    // Record another failure
    transformer._record_failure().await;
    assert!(!transformer._is_circuit_open().await);
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_circuit_breaker_properties(
          failure_threshold in 0..10usize,
          reset_timeout_ms in 10..1000u64
      ) {
          let transformer = CircuitBreakerTransformer::<i32>::new(
              failure_threshold,
              Duration::from_millis(reset_timeout_ms)
          );

          assert_eq!(transformer.failure_threshold, failure_threshold);
          assert_eq!(transformer.reset_timeout, Duration::from_millis(reset_timeout_ms));
          assert_eq!(transformer.failure_count.load(Ordering::SeqCst), 0);
      }

      #[test]
      fn test_circuit_breaker_error_strategies(
          retry_count in 0..10usize
      ) {
          let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100));

          let error = StreamError {
              source: Box::new(std::io::Error::other("property test error")),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "property_test".to_string(),
                  component_type: "CircuitBreakerTransformer".to_string(),
              },
              component: ComponentInfo {
                  name: "property_test".to_string(),
                  type_name: "CircuitBreakerTransformer".to_string(),
              },
              retries: retry_count,
          };

          // Test different error strategies
          let transformer_skip = transformer.clone().with_error_strategy(ErrorStrategy::Skip);
          let transformer_retry = transformer.clone().with_error_strategy(ErrorStrategy::Retry(5));

          assert!(matches!(
              transformer_skip.handle_error(&error),
              ErrorAction::Skip
          ));

          if retry_count < 5 {
              assert!(matches!(
                  transformer_retry.handle_error(&error),
                  ErrorAction::Retry
              ));
          } else {
              assert!(matches!(
                  transformer_retry.handle_error(&error),
                  ErrorAction::Stop
              ));
          }
      }

      #[test]
      fn test_circuit_breaker_config_persistence(
          name in ".*"
      ) {
          let transformer = CircuitBreakerTransformer::<i32>::new(3, Duration::from_millis(100))
              .with_name(name.clone())
              .with_error_strategy(ErrorStrategy::Skip);

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Skip
          ));
      }
  }

  #[tokio::test]
  async fn test_circuit_breaker_edge_cases() {
    // Test with very large failure threshold
    let transformer = CircuitBreakerTransformer::<i32>::new(1000, Duration::from_millis(100));
    assert!(!transformer._is_circuit_open().await);

    // Test with very small reset timeout
    let transformer = CircuitBreakerTransformer::<i32>::new(1, Duration::from_millis(1));
    assert!(!transformer._is_circuit_open().await);

    transformer._record_failure().await;
    assert!(transformer._is_circuit_open().await);

    // Circuit should reset very quickly
    sleep(Duration::from_millis(10)).await;
    assert!(!transformer._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_stream_processing() {
    let mut transformer = CircuitBreakerTransformer::<i32>::new(2, Duration::from_millis(100));

    // Process a stream
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);

    // Circuit should still be closed after successful processing
    assert!(!transformer._is_circuit_open().await);
  }

  #[tokio::test]
  async fn test_circuit_breaker_reuse() {
    let mut transformer = CircuitBreakerTransformer::<i32>::new(2, Duration::from_millis(100));

    // First use
    let input1 = stream::iter(vec![1, 2]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![1, 2]);

    // Record failures to open circuit
    transformer._record_failure().await;
    transformer._record_failure().await;
    assert!(transformer._is_circuit_open().await);

    // Wait for reset
    sleep(Duration::from_millis(150)).await;
    assert!(!transformer._is_circuit_open().await);

    // Second use after reset
    let input2 = stream::iter(vec![3, 4]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![3, 4]);
  }
}
