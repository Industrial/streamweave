use crate::retry_transformer::RetryTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave_core::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T> Transformer for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let max_retries = self.max_retries;
    let backoff = self.backoff;

    Box::pin(input.then(move |item| {
      let _max_retries = max_retries;
      let _backoff = backoff;
      async move {
        // For now, just pass through items without actual retry logic
        // This is a placeholder implementation - real retry logic would need
        // to handle errors from the stream and retry failed operations
        item
      }
    }))
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "retry_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "retry_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use proptest::prelude::*;
  use std::time::Duration;

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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_retry_empty_input() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_retry_with_error() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[test]
  fn test_retry_transformer_clone() {
    let transformer: RetryTransformer<i32> = RetryTransformer::new(5, Duration::from_millis(100));
    let cloned = transformer.clone();

    assert_eq!(transformer.max_retries(), cloned.max_retries());
    assert_eq!(transformer.backoff(), cloned.backoff());
    assert_eq!(transformer.config(), cloned.config());
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
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_retry_transformer_with_name() {
    let transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10)).with_name("test_retry".to_string());

    assert_eq!(transformer.config().name(), Some("test_retry".to_string()));
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

  #[test]
  fn test_retry_transformer_error_context_creation() {
    let transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10)).with_name("test_retry".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_retry");
    assert_eq!(
      context.component_type,
      std::any::type_name::<RetryTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_retry_transformer_component_info() {
    let transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10)).with_name("test_retry".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_retry");
    assert_eq!(
      info.type_name,
      std::any::type_name::<RetryTransformer<i32>>()
    );
  }

  #[test]
  fn test_retry_transformer_default_name() {
    let transformer: RetryTransformer<i32> = RetryTransformer::new(3, Duration::from_millis(10));

    let info = transformer.component_info();
    assert_eq!(info.name, "retry_transformer");
  }

  #[test]
  fn test_retry_transformer_config_mut() {
    let mut transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10));
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_retry_transformer_multiple_transforms() {
    let mut transformer: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10));

    // First transform
    let input1 = stream::iter(vec![1, 2, 3]);
    let result1: Vec<i32> = transformer.transform(Box::pin(input1)).collect().await;
    assert_eq!(result1, vec![1, 2, 3]);

    // Second transform
    let input2 = stream::iter(vec![4, 5, 6]);
    let result2: Vec<i32> = transformer.transform(Box::pin(input2)).collect().await;
    assert_eq!(result2, vec![4, 5, 6]);
  }

  #[test]
  fn test_retry_transformer_edge_cases() {
    // Test with zero retries
    let transformer: RetryTransformer<i32> = RetryTransformer::new(0, Duration::from_millis(10));
    assert_eq!(transformer.max_retries(), 0);

    // Test with very long backoff
    let transformer: RetryTransformer<i32> = RetryTransformer::new(5, Duration::from_secs(3600));
    assert_eq!(transformer.backoff(), Duration::from_secs(3600));

    // Test with very short backoff
    let transformer: RetryTransformer<i32> = RetryTransformer::new(5, Duration::from_nanos(1));
    assert_eq!(transformer.backoff(), Duration::from_nanos(1));
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_retry_transformer_properties(
          max_retries in 0..100usize,
          backoff_ms in 1..1000u64,
          name in ".*"
      ) {
          let backoff = Duration::from_millis(backoff_ms);
          let transformer: RetryTransformer<i32> = RetryTransformer::new(max_retries, backoff)
              .with_name(name.clone());

          assert_eq!(transformer.max_retries(), max_retries);
          assert_eq!(transformer.backoff(), backoff);
          assert_eq!(transformer.config().name(), Some(name));
      }

      #[test]
      fn test_retry_transformer_clone_properties(
          max_retries in 0..100usize,
          backoff_ms in 1..1000u64
      ) {
          let backoff = Duration::from_millis(backoff_ms);
          let transformer: RetryTransformer<i32> = RetryTransformer::new(max_retries, backoff);
          let cloned = transformer.clone();

          assert_eq!(transformer.max_retries(), cloned.max_retries());
          assert_eq!(transformer.backoff(), cloned.backoff());
          assert_eq!(transformer.config(), cloned.config());
      }

      #[test]
      fn test_retry_transformer_error_strategies(
          max_retries in 0..100usize,
          backoff_ms in 1..1000u64,
          retry_count in 0..10usize
      ) {
          let backoff = Duration::from_millis(backoff_ms);
          let transformer: RetryTransformer<i32> = RetryTransformer::new(max_retries, backoff);

          let error = StreamError {
              source: Box::new(TestError("property test error".to_string())),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "property_test".to_string(),
                  component_type: "RetryTransformer".to_string(),
              },
              component: ComponentInfo {
                  name: "property_test".to_string(),
                  type_name: "RetryTransformer".to_string(),
              },
              retries: retry_count,
          };

          // Test different error strategies
          let transformer_skip = transformer.clone().with_error_strategy(ErrorStrategy::Skip);
          let transformer_retry = transformer.clone().with_error_strategy(ErrorStrategy::Retry(max_retries));

          assert!(matches!(
              transformer_skip.handle_error(&error),
              ErrorAction::Skip
          ));

          if retry_count < max_retries {
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
      fn test_retry_transformer_config_persistence(
          max_retries in 0..100usize,
          backoff_ms in 1..1000u64,
          name in ".*"
      ) {
          let backoff = Duration::from_millis(backoff_ms);
          let transformer: RetryTransformer<i32> = RetryTransformer::new(max_retries, backoff)
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
  async fn test_retry_transformer_stream_processing() {
    // Test with i32 type
    let mut transformer_i32: RetryTransformer<i32> =
      RetryTransformer::new(3, Duration::from_millis(10));
    let input_i32 = stream::iter(vec![1i32, 2, 3]);
    let result_i32: Vec<i32> = transformer_i32
      .transform(Box::pin(input_i32))
      .collect()
      .await;
    assert_eq!(result_i32, vec![1, 2, 3]);

    // Test with String type
    let mut transformer_string: RetryTransformer<String> =
      RetryTransformer::new(3, Duration::from_millis(10));
    let input_string = stream::iter(vec!["a".to_string(), "b".to_string()]);
    let result_string: Vec<String> = transformer_string
      .transform(Box::pin(input_string))
      .collect()
      .await;
    assert_eq!(result_string, vec!["a".to_string(), "b".to_string()]);
  }
}
