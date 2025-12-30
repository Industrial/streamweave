use futures::{StreamExt, stream};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_transformers::CircuitBreakerTransformer;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn test_circuit_breaker_basic() {
  let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
  let input = stream::iter(vec![1, 2, 3].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_circuit_breaker_empty_input() {
  let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

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
  let transformer_string = CircuitBreakerTransformer::<String>::new(2, Duration::from_millis(100));
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

#[tokio::test]
async fn test_circuit_breaker_stream_processing() {
  let mut transformer = CircuitBreakerTransformer::<i32>::new(2, Duration::from_millis(100));

  // Process a stream
  let input = stream::iter(vec![1, 2, 3, 4, 5]);
  let boxed_input = Box::pin(input);
  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

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
  let result1: Vec<i32> = transformer.transform(boxed_input1).await.collect().await;
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
  let result2: Vec<i32> = transformer.transform(boxed_input2).await.collect().await;
  assert_eq!(result2, vec![3, 4]);
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
