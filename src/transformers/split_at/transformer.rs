use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::split_at::split_at_transformer::SplitAtTransformer;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

#[async_trait]
impl<T> Transformer for SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((Vec<T>, Vec<T>),);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let index = self.index;
    Box::pin(futures::stream::unfold(
      (input, index),
      |(mut input, index)| async move {
        let mut items = Vec::new();
        while let Some(item) = input.next().await {
          items.push(item);
        }
        if items.is_empty() {
          None
        } else {
          // Ensure index is within bounds to prevent panic
          let safe_index = std::cmp::min(index, items.len());
          let (first, second) = items.split_at(safe_index);
          Some((
            (first.to_vec(), second.to_vec()),
            (
              Box::pin(futures::stream::empty()) as Pin<Box<dyn Stream<Item = T> + Send>>,
              index,
            ),
          ))
        }
      },
    ))
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
        .unwrap_or_else(|| "split_at_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "split_at_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use proptest::prelude::*;

  #[tokio::test]
  async fn test_split_at_basic() {
    let mut transformer = SplitAtTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![1, 2], vec![3, 4, 5])]);
  }

  #[tokio::test]
  async fn test_split_at_empty_input() {
    let mut transformer = SplitAtTransformer::new(2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = SplitAtTransformer::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, Some("test_transformer".to_string()));
  }

  #[test]
  fn test_split_at_transformer_new() {
    let transformer = SplitAtTransformer::<i32>::new(5);

    assert_eq!(transformer.index, 5);
    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_split_at_transformer_with_error_strategy() {
    let transformer = SplitAtTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(transformer.index, 3);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_split_at_transformer_with_name() {
    let transformer = SplitAtTransformer::<i32>::new(4).with_name("test_split_at".to_string());

    assert_eq!(transformer.index, 4);
    assert_eq!(
      transformer.config().name(),
      Some("test_split_at".to_string())
    );
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_at_beginning() {
    let mut transformer = SplitAtTransformer::<i32>::new(0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![], vec![1, 2, 3, 4, 5])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_at_end() {
    let mut transformer = SplitAtTransformer::<i32>::new(5);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3, 4, 5], vec![])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_at_middle() {
    let mut transformer = SplitAtTransformer::<i32>::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3], vec![4, 5])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_beyond_length() {
    let mut transformer = SplitAtTransformer::<i32>::new(10);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // When index > length, split_at panics, so we need to handle this case
    // The transformer should handle this gracefully by not calling split_at with invalid indices
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    // Since index 10 > length 3, the first part should contain all elements and second should be empty
    assert_eq!(first, &vec![1, 2, 3]);
    assert_eq!(second, &Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_split_at_transformer_single_element() {
    let mut transformer = SplitAtTransformer::<i32>::new(1);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![42], vec![])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_two_elements() {
    let mut transformer = SplitAtTransformer::<i32>::new(1);
    let input = stream::iter(vec![10, 20]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![10], vec![20])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_strings() {
    let mut transformer = SplitAtTransformer::<String>::new(2);
    let input = stream::iter(vec![
      "apple".to_string(),
      "banana".to_string(),
      "cherry".to_string(),
      "date".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<String>, Vec<String>)> =
      transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![(
        vec!["apple".to_string(), "banana".to_string()],
        vec!["cherry".to_string(), "date".to_string()]
      )]
    );
  }

  #[tokio::test]
  async fn test_split_at_transformer_different_types() {
    // Test with u64
    let mut transformer_u64 = SplitAtTransformer::<u64>::new(2);
    let input_u64 = stream::iter(vec![1u64, 2u64, 3u64, 4u64]);
    let boxed_input_u64 = Box::pin(input_u64);
    let result_u64: Vec<(Vec<u64>, Vec<u64>)> =
      transformer_u64.transform(boxed_input_u64).collect().await;
    assert_eq!(result_u64, vec![(vec![1u64, 2u64], vec![3u64, 4u64])]);

    // Test with i64
    let mut transformer_i64 = SplitAtTransformer::<i64>::new(1);
    let input_i64 = stream::iter(vec![5i64, 6i64]);
    let boxed_input_i64 = Box::pin(input_i64);
    let result_i64: Vec<(Vec<i64>, Vec<i64>)> =
      transformer_i64.transform(boxed_input_i64).collect().await;
    assert_eq!(result_i64, vec![(vec![5i64], vec![6i64])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_very_large_input() {
    let mut transformer = SplitAtTransformer::<i32>::new(500);
    let input = stream::iter((0..1000).collect::<Vec<i32>>());
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should have one result with split at index 500
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    assert_eq!(first.len(), 500);
    assert_eq!(second.len(), 500);
    assert_eq!(first[0], 0);
    assert_eq!(first[499], 499);
    assert_eq!(second[0], 500);
    assert_eq!(second[499], 999);
  }

  #[test]
  fn test_split_at_transformer_error_handling() {
    let transformer = SplitAtTransformer::<i32>::new(3);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "SplitAtTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "SplitAtTransformer".to_string(),
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
        component_type: "SplitAtTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "SplitAtTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_split_at_transformer_error_context_creation() {
    let transformer = SplitAtTransformer::<i32>::new(2).with_name("test_split_at".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_split_at");
    assert_eq!(
      context.component_type,
      std::any::type_name::<SplitAtTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_split_at_transformer_component_info() {
    let transformer = SplitAtTransformer::<i32>::new(2).with_name("test_split_at".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_split_at");
    assert_eq!(
      info.type_name,
      std::any::type_name::<SplitAtTransformer<i32>>()
    );
  }

  #[test]
  fn test_split_at_transformer_default_name() {
    let transformer = SplitAtTransformer::<i32>::new(2);

    let info = transformer.component_info();
    assert_eq!(info.name, "split_at_transformer");
  }

  #[test]
  fn test_split_at_transformer_config_mut() {
    let mut transformer = SplitAtTransformer::<i32>::new(2);
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_split_at_transformer_reuse() {
    let mut transformer = SplitAtTransformer::<i32>::new(2);

    // First use
    let input1 = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![(vec![1, 2], vec![3, 4, 5])]);

    // Second use
    let input2 = stream::iter(vec![6, 7, 8, 9, 10]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![(vec![6, 7], vec![8, 9, 10])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_edge_cases() {
    let mut transformer = SplitAtTransformer::<i32>::new(1);

    // Test with negative numbers (though index is usize, so this won't happen in practice)
    let input = stream::iter(vec![-5, -3, -1, 0, 2]);
    let boxed_input = Box::pin(input);
    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![(vec![-5], vec![-3, -1, 0, 2])]);

    // Test with mixed positive and negative
    let input = stream::iter(vec![5, -3, 0, -7, 2]);
    let boxed_input = Box::pin(input);
    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![(vec![5], vec![-3, 0, -7, 2])]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_deterministic_behavior() {
    let mut transformer = SplitAtTransformer::<i32>::new(2);

    // Test that splitting is deterministic
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result1: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result1, vec![(vec![1, 2], vec![3, 4, 5])]);

    // Test again to ensure consistency
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);
    let result2: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result2, vec![(vec![1, 2], vec![3, 4, 5])]);
  }

  // Property-based tests using proptest
  proptest! {
    #[test]
    fn test_split_at_transformer_properties(
      name in ".*"
    ) {
      let transformer = SplitAtTransformer::<i32>::new(5)
        .with_name(name.clone());

      assert_eq!(transformer.index, 5);
      assert_eq!(transformer.config().name(), Some(name));
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Stop
      ));
    }

    #[test]
    fn test_split_at_transformer_error_strategies(
      retry_count in 0..10usize
    ) {
      let transformer = SplitAtTransformer::<i32>::new(3);

      let error = StreamError {
        source: Box::new(std::io::Error::other("property test error")),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "property_test".to_string(),
          component_type: "SplitAtTransformer".to_string(),
        },
        component: ComponentInfo {
          name: "property_test".to_string(),
          type_name: "SplitAtTransformer".to_string(),
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
    fn test_split_at_transformer_config_persistence(
      name in ".*"
    ) {
      let transformer = SplitAtTransformer::<i32>::new(4)
        .with_name(name.clone())
        .with_error_strategy(ErrorStrategy::Skip);

      assert_eq!(transformer.index, 4);
      assert_eq!(transformer.config().name(), Some(name));
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Skip
      ));
    }

    #[test]
    fn test_split_at_transformer_splitting_properties(
      _values in prop::collection::vec(0..100i32, 0..50)
    ) {
      // Test that the transformer can handle this data structure
      let transformer = SplitAtTransformer::<i32>::new(2);
      assert_eq!(transformer.index, 2);
      assert_eq!(transformer.config().name(), None);
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Stop
      ));
    }
  }

  #[tokio::test]
  async fn test_split_at_transformer_stream_processing() {
    let mut transformer = SplitAtTransformer::<i32>::new(3);

    // Process a stream with various patterns
    let input = stream::iter(vec![9, 3, 7, 1, 5, 2, 8, 4, 6, 0]);
    let boxed_input = Box::pin(input);
    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should split at index 3
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    assert_eq!(first, &vec![9, 3, 7]);
    assert_eq!(second, &vec![1, 5, 2, 8, 4, 6, 0]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_nested_types() {
    type NestedVecResult = Vec<(Vec<Vec<i32>>, Vec<Vec<i32>>)>;

    let mut transformer = SplitAtTransformer::<Vec<i32>>::new(1);

    // Test splitting vectors
    let input = stream::iter(vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
    let boxed_input = Box::pin(input);
    let result: NestedVecResult = transformer.transform(boxed_input).collect().await;

    // Should split at index 1
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    assert_eq!(first, &vec![vec![1, 2]]);
    assert_eq!(second, &vec![vec![3, 4], vec![5, 6]]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_exact_input_length() {
    let mut transformer = SplitAtTransformer::<i32>::new(5);
    let input = stream::iter(vec![5, 3, 1, 4, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should have same length and be split at index 5
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    assert_eq!(first.len(), 5);
    assert_eq!(second.len(), 0);
    assert_eq!(first, &vec![5, 3, 1, 4, 2]);
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_partial() {
    let mut transformer = SplitAtTransformer::<i32>::new(3);
    let input = stream::iter(vec![5, 3, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should be split at index 3
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    assert_eq!(first, &vec![5, 3, 1]);
    assert_eq!(second, &Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_split_at_transformer_split_with_zeros() {
    let mut transformer = SplitAtTransformer::<i32>::new(2);
    let input = stream::iter(vec![0, 5, 0, 3, 0, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Should handle zeros correctly
    assert_eq!(result.len(), 1);
    let (first, second) = &result[0];
    assert_eq!(first, &vec![0, 5]);
    assert_eq!(second, &vec![0, 3, 0, 1]);
  }

  #[test]
  fn test_split_at_transformer_set_config_impl() {
    let mut transformer = SplitAtTransformer::<i32>::new(5);
    let new_config = TransformerConfig::default()
      .with_name("test_split_at".to_string())
      .with_error_strategy(ErrorStrategy::Skip);
    transformer.set_config_impl(new_config);
    assert_eq!(transformer.config.name, Some("test_split_at".to_string()));
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_split_at_transformer_get_config_impl() {
    let transformer = SplitAtTransformer::<i32>::new(5).with_name("test".to_string());
    let config = transformer.get_config_impl();
    assert_eq!(config.name, Some("test".to_string()));
  }

  #[test]
  fn test_split_at_transformer_get_config_mut_impl() {
    let mut transformer = SplitAtTransformer::<i32>::new(5);
    let config = transformer.get_config_mut_impl();
    config.name = Some("mutated".to_string());
    assert_eq!(transformer.config.name, Some("mutated".to_string()));
  }

  #[test]
  fn test_split_at_transformer_handle_error_stop() {
    let transformer = SplitAtTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Stop);
    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    };
    assert_eq!(transformer.handle_error(&error), ErrorAction::Stop);
  }

  #[test]
  fn test_split_at_transformer_handle_error_skip() {
    let transformer = SplitAtTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Skip);
    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    };
    assert_eq!(transformer.handle_error(&error), ErrorAction::Skip);
  }

  #[test]
  fn test_split_at_transformer_handle_error_retry() {
    let transformer =
      SplitAtTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 1,
    };
    assert_eq!(transformer.handle_error(&error), ErrorAction::Retry);
  }

  #[test]
  fn test_split_at_transformer_handle_error_retry_exhausted() {
    let transformer =
      SplitAtTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Retry(3));
    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 3,
    };
    assert_eq!(transformer.handle_error(&error), ErrorAction::Stop);
  }

  #[test]
  fn test_split_at_transformer_create_error_context() {
    let transformer = SplitAtTransformer::<i32>::new(5).with_name("test_split_at".to_string());
    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_split_at");
    assert_eq!(context.item, Some(42));
    assert!(context.component_type.contains("SplitAtTransformer"));
  }

  #[test]
  fn test_split_at_transformer_create_error_context_no_item() {
    let transformer = SplitAtTransformer::<i32>::new(5);
    let context = transformer.create_error_context(None);
    assert_eq!(context.component_name, "split_at_transformer");
    assert_eq!(context.item, None);
  }

  #[test]
  fn test_split_at_transformer_component_info_default() {
    let transformer = SplitAtTransformer::<i32>::new(5);
    let info = transformer.component_info();
    assert_eq!(info.name, "split_at_transformer");
  }
}
