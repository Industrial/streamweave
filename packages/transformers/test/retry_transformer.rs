use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave_transformers::Input;
use streamweave_transformers::RetryTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;
  use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

  #[derive(Debug)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_retry_basic() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_retry_empty_input() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_retry_transformer_clone() {
    let transformer: RetryTransformer<i32> = RetryTransformer::new(5, Duration::from_millis(100));
    let cloned = transformer.clone();

    assert_eq!(transformer.max_retries(), cloned.max_retries());
    assert_eq!(transformer.backoff(), cloned.backoff());
  }

  #[test]
  fn test_retry_transformer_getters() {
    let transformer: RetryTransformer<i32> = RetryTransformer::new(7, Duration::from_millis(200));

    assert_eq!(transformer.max_retries(), 7);
    assert_eq!(transformer.backoff(), Duration::from_millis(200));
  }

  #[test]
  fn test_retry_transformer_with_error_strategy() {
    let transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10)).with_error_strategy(ErrorStrategy::Skip);

    assert!(matches!(
      transformer.get_config_impl().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_retry_transformer_with_name() {
    let transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10)).with_name("test_retry".to_string());

    assert_eq!(
      transformer.get_config_impl().name(),
      Some("test_retry".to_string())
    );
  }

  #[test]
  fn test_retry_transformer_error_handling() {
    let transformer: RetryTransformer<i32> = RetryTransformer::new(3, Duration::from_millis(10));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RetryTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RetryTransformer".to_string(),
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
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RetryTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RetryTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }
}
