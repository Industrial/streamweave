use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::sort::sort_transformer::SortTransformer;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

#[async_trait]
impl<T> Transformer for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(futures::stream::unfold(input, |mut input| async move {
      let mut items = Vec::new();
      while let Some(item) = input.next().await {
        items.push(item);
      }
      if items.is_empty() {
        None
      } else {
        items.sort();
        Some((
          items[0].clone(),
          (Box::pin(futures::stream::iter(items[1..].to_vec()))
            as Pin<Box<dyn Stream<Item = T> + Send>>),
        ))
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
        .unwrap_or_else(|| "sort_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "sort_transformer".to_string()),
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
  async fn test_sort_basic() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(vec![3, 1, 4, 1, 5, 9]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 1, 3, 4, 5, 9]);
  }

  #[tokio::test]
  async fn test_sort_empty_input() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = SortTransformer::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, Some("test_transformer".to_string()));
  }

  #[test]
  fn test_sort_transformer_new() {
    let transformer = SortTransformer::<i32>::new();

    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_sort_transformer_with_error_strategy() {
    let transformer = SortTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_sort_transformer_with_name() {
    let transformer = SortTransformer::<i32>::new().with_name("test_sort".to_string());

    assert_eq!(transformer.config().name(), Some("test_sort".to_string()));
  }

  #[tokio::test]
  async fn test_sort_transformer_already_sorted() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_sort_transformer_reverse_sorted() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![5, 4, 3, 2, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_sort_transformer_duplicates() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![3, 3, 3, 1, 1, 2, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 1, 2, 2, 3, 3, 3]);
  }

  #[tokio::test]
  async fn test_sort_transformer_single_element() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42]);
  }

  #[tokio::test]
  async fn test_sort_transformer_two_elements() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![5, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 5]);
  }

  #[tokio::test]
  async fn test_sort_transformer_strings() {
    let mut transformer = SortTransformer::<String>::new();
    let input = stream::iter(vec![
      "zebra".to_string(),
      "apple".to_string(),
      "banana".to_string(),
      "cat".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![
        "apple".to_string(),
        "banana".to_string(),
        "cat".to_string(),
        "zebra".to_string()
      ]
    );
  }

  #[tokio::test]
  async fn test_sort_transformer_different_types() {
    // Test with u64
    let mut transformer_u64 = SortTransformer::<u64>::new();
    let input_u64 = stream::iter(vec![5u64, 1u64, 3u64, 2u64, 4u64]);
    let boxed_input_u64 = Box::pin(input_u64);
    let result_u64: Vec<u64> = transformer_u64.transform(boxed_input_u64).collect().await;
    assert_eq!(result_u64, vec![1u64, 2u64, 3u64, 4u64, 5u64]);

    // Test with i64 (which implements Ord)
    let mut transformer_i64 = SortTransformer::<i64>::new();
    let input_i64 = stream::iter(vec![3i64, 1i64, 2i64, 0i64]);
    let boxed_input_i64 = Box::pin(input_i64);
    let result_i64: Vec<i64> = transformer_i64.transform(boxed_input_i64).collect().await;
    assert_eq!(result_i64, vec![0i64, 1i64, 2i64, 3i64]);
  }

  #[tokio::test]
  async fn test_sort_transformer_very_large_input() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter((0..1000).rev().collect::<Vec<i32>>());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should be sorted from 0 to 999
    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], 0);
    assert_eq!(result[999], 999);

    // Verify it's properly sorted
    for i in 1..result.len() {
      assert!(result[i] >= result[i - 1]);
    }
  }

  #[test]
  fn test_sort_transformer_error_handling() {
    let transformer = SortTransformer::<i32>::new();

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "SortTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "SortTransformer".to_string(),
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
        component_type: "SortTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "SortTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_sort_transformer_error_context_creation() {
    let transformer = SortTransformer::<i32>::new().with_name("test_sort".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_sort");
    assert_eq!(
      context.component_type,
      std::any::type_name::<SortTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_sort_transformer_component_info() {
    let transformer = SortTransformer::<i32>::new().with_name("test_sort".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_sort");
    assert_eq!(
      info.type_name,
      std::any::type_name::<SortTransformer<i32>>()
    );
  }

  #[test]
  fn test_sort_transformer_default_name() {
    let transformer = SortTransformer::<i32>::new();

    let info = transformer.component_info();
    assert_eq!(info.name, "sort_transformer");
  }

  #[test]
  fn test_sort_transformer_config_mut() {
    let mut transformer = SortTransformer::<i32>::new();
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_sort_transformer_reuse() {
    let mut transformer = SortTransformer::<i32>::new();

    // First use
    let input1 = stream::iter(vec![3, 1, 4, 1, 5, 9]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![1, 1, 3, 4, 5, 9]);

    // Second use
    let input2 = stream::iter(vec![8, 2, 7, 1, 9, 0]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![0, 1, 2, 7, 8, 9]);
  }

  #[tokio::test]
  async fn test_sort_transformer_edge_cases() {
    let mut transformer = SortTransformer::<i32>::new();

    // Test with negative numbers
    let input = stream::iter(vec![-5, -1, -10, -2, -8]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![-10, -8, -5, -2, -1]);

    // Test with mixed positive and negative
    let input = stream::iter(vec![5, -3, 0, -7, 2]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![-7, -3, 0, 2, 5]);
  }

  #[tokio::test]
  async fn test_sort_transformer_deterministic_behavior() {
    let mut transformer = SortTransformer::<i32>::new();

    // Test that sorting is deterministic
    let input = stream::iter(vec![3, 1, 4, 1, 5, 9]);
    let boxed_input = Box::pin(input);

    let result1: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result1, vec![1, 1, 3, 4, 5, 9]);

    // Test again to ensure consistency
    let input = stream::iter(vec![3, 1, 4, 1, 5, 9]);
    let boxed_input = Box::pin(input);
    let result2: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result2, vec![1, 1, 3, 4, 5, 9]);
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_sort_transformer_properties(
          name in ".*"
      ) {
          let transformer = SortTransformer::<i32>::new()
              .with_name(name.clone());

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Stop
          ));
      }

      #[test]
      fn test_sort_transformer_error_strategies(
          retry_count in 0..10usize
      ) {
          let transformer = SortTransformer::<i32>::new();

          let error = StreamError {
              source: Box::new(std::io::Error::other("property test error")),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "property_test".to_string(),
                  component_type: "SortTransformer".to_string(),
              },
              component: ComponentInfo {
                  name: "property_test".to_string(),
                  type_name: "SortTransformer".to_string(),
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
      fn test_sort_transformer_config_persistence(
          name in ".*"
      ) {
          let transformer = SortTransformer::<i32>::new()
              .with_name(name.clone())
              .with_error_strategy(ErrorStrategy::Skip);

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Skip
          ));
      }

      #[test]
      fn test_sort_transformer_sorting_properties(
          _values in prop::collection::vec(0..100i32, 0..50)
      ) {
          // Test that the transformer can handle this data structure
          let transformer = SortTransformer::<i32>::new();
          assert_eq!(transformer.config().name(), None);
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Stop
          ));
      }
  }

  #[tokio::test]
  async fn test_sort_transformer_stream_processing() {
    let mut transformer = SortTransformer::<i32>::new();

    // Process a stream with various patterns
    let input = stream::iter(vec![9, 3, 7, 1, 5, 2, 8, 4, 6, 0]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should be sorted
    assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  }

  #[tokio::test]
  async fn test_sort_transformer_nested_types() {
    let mut transformer = SortTransformer::<Vec<i32>>::new();

    // Test sorting vectors
    let input = stream::iter(vec![
      vec![3, 2, 1],
      vec![1, 2, 3],
      vec![2, 1, 3],
      vec![1, 1, 1],
    ]);
    let boxed_input = Box::pin(input);
    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Vectors should be sorted lexicographically
    assert_eq!(
      result,
      vec![vec![1, 1, 1], vec![1, 2, 3], vec![2, 1, 3], vec![3, 2, 1]]
    );
  }

  #[tokio::test]
  async fn test_sort_transformer_sort_exact_input_length() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![5, 3, 1, 4, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should have same length and be sorted
    assert_eq!(result.len(), 5);
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_sort_transformer_sort_partial() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![5, 3, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should be sorted
    assert_eq!(result, vec![1, 3, 5]);
  }

  #[tokio::test]
  async fn test_sort_transformer_sort_with_zeros() {
    let mut transformer = SortTransformer::<i32>::new();
    let input = stream::iter(vec![0, 5, 0, 3, 0, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should handle zeros correctly
    assert_eq!(result, vec![0, 0, 0, 1, 3, 5]);
  }
}
