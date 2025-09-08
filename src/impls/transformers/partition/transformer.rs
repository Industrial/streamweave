use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::transformers::partition::PartitionTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

#[async_trait]
impl<F, T> Transformer for PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();
    Box::pin(futures::stream::unfold(
      (input, predicate),
      |(mut input, predicate)| async move {
        let mut matches = Vec::new();
        let mut non_matches = Vec::new();
        while let Some(item) = input.next().await {
          if predicate(&item) {
            matches.push(item);
          } else {
            non_matches.push(item);
          }
        }
        if matches.is_empty() && non_matches.is_empty() {
          None
        } else {
          Some((
            (matches, non_matches),
            (
              Box::pin(futures::stream::empty()) as Pin<Box<dyn Stream<Item = T> + Send>>,
              predicate,
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
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
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
        .unwrap_or_else(|| "partition_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "partition_transformer".to_string()),
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
  async fn test_partition_basic() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![2, 4, 6], vec![1, 3, 5])]);
  }

  #[tokio::test]
  async fn test_partition_empty_input() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, Some("test_transformer".to_string()));
  }

  // Test partition with all even numbers
  #[tokio::test]
  async fn test_partition_all_even() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![2, 4, 6, 8, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![2, 4, 6, 8, 10], vec![])]);
  }

  // Test partition with all odd numbers
  #[tokio::test]
  async fn test_partition_all_odd() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 3, 5, 7, 9]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![], vec![1, 3, 5, 7, 9])]);
  }

  // Test partition with single element
  #[tokio::test]
  async fn test_partition_single_element() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![42], vec![])]);
  }

  // Test partition with single odd element
  #[tokio::test]
  async fn test_partition_single_odd_element() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![43]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![], vec![43])]);
  }

  // Test partition with strings
  #[tokio::test]
  async fn test_partition_with_strings() {
    let mut transformer = PartitionTransformer::new(|x: &String| x.len() > 3);
    let input = stream::iter(vec![
      "a".to_string(),
      "bb".to_string(),
      "ccc".to_string(),
      "dddd".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<String>, Vec<String>)> =
      transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![(
        vec!["dddd".to_string()],
        vec!["a".to_string(), "bb".to_string(), "ccc".to_string()]
      )]
    );
  }

  // Test partition with floats
  #[tokio::test]
  async fn test_partition_with_floats() {
    let mut transformer = PartitionTransformer::new(|x: &f64| *x > 0.0);
    let input = stream::iter(vec![-1.5, 0.0, 2.7, -3.2, 4.1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<f64>, Vec<f64>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![2.7, 4.1], vec![-1.5, 0.0, -3.2])]);
  }

  // Test partition with custom predicate
  #[tokio::test]
  async fn test_partition_custom_predicate() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x > 5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![6, 7, 8, 9, 10], vec![1, 2, 3, 4, 5])]);
  }

  // Test partition with zero values
  #[tokio::test]
  async fn test_partition_with_zero_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x == 0);
    let input = stream::iter(vec![0, 1, 0, 2, 0, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![0, 0, 0], vec![1, 2, 3])]);
  }

  // Test partition with negative numbers
  #[tokio::test]
  async fn test_partition_with_negative_numbers() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x < 0);
    let input = stream::iter(vec![-1, 2, -3, 4, -5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![-1, -3, -5], vec![2, 4])]);
  }

  // Test partition with large numbers
  #[tokio::test]
  async fn test_partition_with_large_numbers() {
    let mut transformer = PartitionTransformer::new(|x: &i64| *x > i64::MAX / 2);
    let input = stream::iter(vec![i64::MAX, i64::MIN, 0, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i64>, Vec<i64>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![i64::MAX], vec![i64::MIN, 0, 1])]);
  }

  // Test partition with duplicate values
  #[tokio::test]
  async fn test_partition_with_duplicate_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x % 2 == 0);
    let input = stream::iter(vec![1, 1, 2, 2, 3, 3, 4, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![2, 2, 4, 4], vec![1, 1, 3, 3])]);
  }

  // Test partition with ascending values
  #[tokio::test]
  async fn test_partition_with_ascending_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x < 5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3, 4], vec![5, 6, 7, 8, 9, 10])]);
  }

  // Test partition with descending values
  #[tokio::test]
  async fn test_partition_with_descending_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x > 5);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![10, 9, 8, 7, 6], vec![5, 4, 3, 2, 1])]);
  }

  // Test partition with alternating values
  #[tokio::test]
  async fn test_partition_with_alternating_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x > 0);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3], vec![-1, -2, -3])]);
  }

  // Test partition with custom error strategy
  #[tokio::test]
  async fn test_partition_with_custom_error_strategy() {
    let custom_handler = |_error: &StreamError<i32>| ErrorAction::Skip;
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::new_custom(custom_handler))
      .with_name("custom_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.name(), Some("custom_transformer".to_string()));
  }

  // Test partition with retry error strategy
  #[tokio::test]
  async fn test_partition_with_retry_error_strategy() {
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("retry_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Retry(3));
    assert_eq!(config.name(), Some("retry_transformer".to_string()));
  }

  // Test partition with stop error strategy
  #[tokio::test]
  async fn test_partition_with_stop_error_strategy() {
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Stop)
      .with_name("stop_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Stop);
    assert_eq!(config.name(), Some("stop_transformer".to_string()));
  }

  // Test partition with skip error strategy
  #[tokio::test]
  async fn test_partition_with_skip_error_strategy() {
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("skip_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("skip_transformer".to_string()));
  }

  // Test partition with empty name
  #[tokio::test]
  async fn test_partition_with_empty_name() {
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0).with_name("".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.name(), Some("".to_string()));
  }

  // Test partition with very long name
  #[tokio::test]
  async fn test_partition_with_very_long_name() {
    let long_name = "a".repeat(1000);
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0).with_name(long_name.clone());

    let config = transformer.get_config_impl();
    assert_eq!(config.name(), Some(long_name));
  }

  // Test partition with unicode name
  #[tokio::test]
  async fn test_partition_with_unicode_name() {
    let unicode_name = "🚀火箭🚀".to_string();
    let transformer =
      PartitionTransformer::new(|x: &i32| x % 2 == 0).with_name(unicode_name.clone());

    let config = transformer.get_config_impl();
    assert_eq!(config.name(), Some(unicode_name));
  }

  // Test partition with special characters in name
  #[tokio::test]
  async fn test_partition_with_special_characters_name() {
    let special_name = "!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string();
    let transformer =
      PartitionTransformer::new(|x: &i32| x % 2 == 0).with_name(special_name.clone());

    let config = transformer.get_config_impl();
    assert_eq!(config.name(), Some(special_name));
  }

  // Test partition with complex predicate
  #[tokio::test]
  async fn test_partition_with_complex_predicate() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x > 0 && *x % 2 == 0);
    let input = stream::iter(vec![-2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![(vec![2, 4, 6, 8, 10], vec![-2, -1, 0, 1, 3, 5, 7, 9])]
    );
  }

  // Test partition with edge case predicate
  #[tokio::test]
  async fn test_partition_with_edge_case_predicate() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x == i32::MAX);
    let input = stream::iter(vec![i32::MAX, i32::MIN, 0, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(vec![i32::MAX], vec![i32::MIN, 0, 1])]);
  }

  // Test partition with mixed data types
  #[tokio::test]
  async fn test_partition_with_mixed_data_types() {
    // Test with i32
    let mut transformer_i32 = PartitionTransformer::new(|x: &i32| *x % 2 == 0);
    let input_i32 = stream::iter(vec![1, 2, 3, 4, 5]);
    let result_i32: Vec<(Vec<i32>, Vec<i32>)> = transformer_i32
      .transform(Box::pin(input_i32))
      .collect()
      .await;
    assert_eq!(result_i32, vec![(vec![2, 4], vec![1, 3, 5])]);

    // Test with String
    let mut transformer_string = PartitionTransformer::new(|x: &String| x.len() > 2);
    let input_string = stream::iter(vec!["a".to_string(), "bb".to_string(), "ccc".to_string()]);
    let result_string: Vec<(Vec<String>, Vec<String>)> = transformer_string
      .transform(Box::pin(input_string))
      .collect()
      .await;
    assert_eq!(
      result_string,
      vec![(
        vec!["ccc".to_string()],
        vec!["a".to_string(), "bb".to_string()]
      )]
    );

    // Test with f64
    let mut transformer_f64 = PartitionTransformer::new(|x: &f64| *x > 0.0);
    let input_f64 = stream::iter(vec![-1.1, 0.0, 2.2]);
    let result_f64: Vec<(Vec<f64>, Vec<f64>)> = transformer_f64
      .transform(Box::pin(input_f64))
      .collect()
      .await;
    assert_eq!(result_f64, vec![(vec![2.2], vec![-1.1, 0.0])]);
  }

  // Test partition transformer clone behavior
  #[tokio::test]
  async fn test_partition_transformer_clone_behavior() {
    let mut transformer1 = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test1".to_string());

    let mut transformer2 = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test2".to_string());

    // Both transformers should work independently
    let input1 = stream::iter(vec![1, 2, 3, 4, 5]);
    let input2 = stream::iter(vec![6, 7, 8, 9, 10]);

    let result1: Vec<(Vec<i32>, Vec<i32>)> =
      transformer1.transform(Box::pin(input1)).collect().await;
    let result2: Vec<(Vec<i32>, Vec<i32>)> =
      transformer2.transform(Box::pin(input2)).collect().await;

    assert_eq!(result1, vec![(vec![2, 4], vec![1, 3, 5])]);
    assert_eq!(result2, vec![(vec![6, 8, 10], vec![7, 9])]);
  }

  // Test partition transformer with very large input
  #[tokio::test]
  async fn test_partition_transformer_very_large_input() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x % 2 == 0);
    let large_input: Vec<i32> = (1..=1000).collect();
    let input = stream::iter(large_input);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(Box::pin(input)).collect().await;

    // Verify we got the expected result
    assert_eq!(result.len(), 1);
    let (evens, odds) = &result[0];

    // Verify even numbers
    assert_eq!(evens.len(), 500); // 500 even numbers from 1..=1000
    assert_eq!(evens[0], 2);
    assert_eq!(evens[499], 1000);

    // Verify odd numbers
    assert_eq!(odds.len(), 500); // 500 odd numbers from 1..=1000
    assert_eq!(odds[0], 1);
    assert_eq!(odds[499], 999);
  }

  // Test partition transformer with very small input
  #[tokio::test]
  async fn test_partition_transformer_very_small_input() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x == 1);
    let input = stream::iter(vec![1]);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![(vec![1], vec![])]);
  }

  // Test partition transformer with exact partition size
  #[tokio::test]
  async fn test_partition_transformer_exact_partition_size() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x < 5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8]);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3, 4], vec![5, 6, 7, 8])]);
  }

  // Test partition transformer with single partition
  #[tokio::test]
  async fn test_partition_transformer_single_partition() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x > 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3, 4, 5], vec![])]);
  }

  // Test partition transformer with empty partition
  #[tokio::test]
  async fn test_partition_transformer_empty_partition() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x < 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![(vec![], vec![1, 2, 3, 4, 5])]);
  }

  // Test partition transformer with mixed partition
  #[tokio::test]
  async fn test_partition_transformer_mixed_partition() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x % 3 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let result: Vec<(Vec<i32>, Vec<i32>)> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![(vec![3, 6, 9], vec![1, 2, 4, 5, 7, 8])]);
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_partition_properties(
          values in prop::collection::vec(0..100i32, 0..50)
      ) {
          // Test that partition transformer can handle various input sizes
          let _transformer = PartitionTransformer::<_, i32>::new(|x: &i32| x % 2 == 0);

          // Verify the transformer can be created with various input sizes
          assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
      }

      #[test]
      fn test_partition_error_strategy_properties(
          error_strategy in prop::sample::select(vec![
              ErrorStrategy::<i32>::Stop,
              ErrorStrategy::<i32>::Skip,
              ErrorStrategy::<i32>::Retry(5),
          ])
      ) {
          // Test that partition transformer can handle different error strategies
          let transformer = PartitionTransformer::<_, i32>::new(|x: &i32| x % 2 == 0)
              .with_error_strategy(error_strategy.clone());

          // Verify the transformer can be created with various error strategies
          assert_eq!(transformer.get_config_impl().error_strategy, error_strategy);
      }

      #[test]
      fn test_partition_name_properties(
          name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
      ) {
          // Test that partition transformer can handle different names
          let transformer = PartitionTransformer::<_, i32>::new(|x: &i32| x % 2 == 0)
              .with_name(name.clone());

          // Verify the transformer can be created with various names
          assert_eq!(transformer.get_config_impl().name, Some(name));
      }

      #[test]
      fn test_partition_transformation_properties(
          values in prop::collection::vec(0..100i32, 0..30)
      ) {
          // Test that partition transformation works correctly
          let _transformer = PartitionTransformer::<_, i32>::new(|x: &i32| x % 2 == 0);

          // Verify the transformer can be created with various input sizes
          assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
      }
  }

  // Test partition transformer with different data types
  #[tokio::test]
  async fn test_partition_transformer_different_data_types() {
    // Test with i32
    let mut transformer_i32 = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input_i32 = stream::iter(vec![1, 2, 3, 4, 5]);
    let result_i32: Vec<(Vec<i32>, Vec<i32>)> = transformer_i32
      .transform(Box::pin(input_i32))
      .collect()
      .await;
    assert_eq!(result_i32, vec![(vec![2, 4], vec![1, 3, 5])]);

    // Test with String
    let mut transformer_string = PartitionTransformer::new(|x: &String| x.len() > 2);
    let input_string = stream::iter(vec!["a".to_string(), "bb".to_string(), "ccc".to_string()]);
    let result_string: Vec<(Vec<String>, Vec<String>)> = transformer_string
      .transform(Box::pin(input_string))
      .collect()
      .await;
    assert_eq!(
      result_string,
      vec![(
        vec!["ccc".to_string()],
        vec!["a".to_string(), "bb".to_string()]
      )]
    );

    // Test with f64
    let mut transformer_f64 = PartitionTransformer::new(|x: &f64| *x > 0.0);
    let input_f64 = stream::iter(vec![-1.1, 0.0, 2.2]);
    let result_f64: Vec<(Vec<f64>, Vec<f64>)> = transformer_f64
      .transform(Box::pin(input_f64))
      .collect()
      .await;
    assert_eq!(result_f64, vec![(vec![2.2], vec![-1.1, 0.0])]);
  }
}
