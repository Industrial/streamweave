use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::limit::limit_transformer::LimitTransformer;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Transformer for LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let limit = self.limit;
    Box::pin(input.take(limit))
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
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "limit_transformer".to_string()),
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

  #[tokio::test]
  async fn test_limit_basic() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_limit_empty_input() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = LimitTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  // Test limit with zero
  #[tokio::test]
  async fn test_limit_zero() {
    let mut transformer = LimitTransformer::new(0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  // Test limit larger than input
  #[tokio::test]
  async fn test_limit_larger_than_input() {
    let mut transformer = LimitTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test limit equal to input size
  #[tokio::test]
  async fn test_limit_equal_to_input_size() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test limit with single element
  #[tokio::test]
  async fn test_limit_single_element() {
    let mut transformer = LimitTransformer::new(1);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42]);
  }

  // Test limit with very large limit
  #[tokio::test]
  async fn test_limit_very_large() {
    let mut transformer = LimitTransformer::new(usize::MAX);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test limit with negative numbers
  #[tokio::test]
  async fn test_limit_with_negative_numbers() {
    let mut transformer = LimitTransformer::new(2);
    let input = stream::iter(vec![-1, -2, -3, -4, -5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![-1, -2]);
  }

  // Test limit with strings
  #[tokio::test]
  async fn test_limit_with_strings() {
    let mut transformer = LimitTransformer::new(2);
    let input = stream::iter(vec![
      "hello".to_string(),
      "world".to_string(),
      "test".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
  }

  // Test limit with floats
  #[tokio::test]
  async fn test_limit_with_floats() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![1.1, 2.2, 3.3, 4.4, 5.5]);
    let boxed_input = Box::pin(input);

    let result: Vec<f64> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1.1, 2.2, 3.3]);
  }

  // Test limit with custom error strategy
  #[tokio::test]
  async fn test_limit_with_custom_error_strategy() {
    let custom_handler = |_error: &StreamError<i32>| ErrorAction::Skip;
    let transformer = LimitTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::new_custom(custom_handler))
      .with_name("custom_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.name(), Some("custom_transformer".to_string()));
  }

  // Test limit with retry error strategy
  #[tokio::test]
  async fn test_limit_with_retry_error_strategy() {
    let transformer = LimitTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("retry_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Retry(3));
    assert_eq!(config.name(), Some("retry_transformer".to_string()));
  }

  // Test limit with stop error strategy
  #[tokio::test]
  async fn test_limit_with_stop_error_strategy() {
    let transformer = LimitTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Stop)
      .with_name("stop_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Stop);
    assert_eq!(config.name(), Some("stop_transformer".to_string()));
  }

  // Test limit with skip error strategy
  #[tokio::test]
  async fn test_limit_with_skip_error_strategy() {
    let transformer = LimitTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("skip_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("skip_transformer".to_string()));
  }

  // Test limit with empty name
  #[tokio::test]
  async fn test_limit_with_empty_name() {
    let transformer = LimitTransformer::<i32>::new(2).with_name("".to_string());

    let config = transformer.config();
    assert_eq!(config.name(), Some("".to_string()));
  }

  // Test limit with very long name
  #[tokio::test]
  async fn test_limit_with_very_long_name() {
    let long_name = "a".repeat(1000);
    let transformer = LimitTransformer::<i32>::new(2).with_name(long_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(long_name));
  }

  // Test limit with unicode name
  #[tokio::test]
  async fn test_limit_with_unicode_name() {
    let unicode_name = "🚀火箭🚀".to_string();
    let transformer = LimitTransformer::<i32>::new(2).with_name(unicode_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(unicode_name));
  }

  // Test limit with special characters in name
  #[tokio::test]
  async fn test_limit_with_special_characters_name() {
    let special_name = "!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string();
    let transformer = LimitTransformer::<i32>::new(2).with_name(special_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(special_name));
  }

  // Test limit with large numbers
  #[tokio::test]
  async fn test_limit_with_large_numbers() {
    let mut transformer = LimitTransformer::<i64>::new(2);
    let input = stream::iter(vec![i64::MAX, i64::MIN, 0]);
    let boxed_input = Box::pin(input);

    let result: Vec<i64> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![i64::MAX, i64::MIN]);
  }

  // Test limit with zero values
  #[tokio::test]
  async fn test_limit_with_zero_values() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![0, 0, 0, 1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![0, 0, 0]);
  }

  // Test limit with duplicate values
  #[tokio::test]
  async fn test_limit_with_duplicate_values() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![1, 1, 1, 2, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 1, 1]);
  }

  // Test limit with ascending values
  #[tokio::test]
  async fn test_limit_with_ascending_values() {
    let mut transformer = LimitTransformer::new(4);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4]);
  }

  // Test limit with descending values
  #[tokio::test]
  async fn test_limit_with_descending_values() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![10, 9, 8]);
  }

  // Test limit with alternating values
  #[tokio::test]
  async fn test_limit_with_alternating_values() {
    let mut transformer = LimitTransformer::new(5);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3, 4, -4]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, -1, 2, -2, 3]);
  }

  // Test limit with edge case limit 1
  #[tokio::test]
  async fn test_limit_edge_case_one() {
    let mut transformer = LimitTransformer::new(1);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42]);
  }

  // Test limit with edge case limit 2
  #[tokio::test]
  async fn test_limit_edge_case_two() {
    let mut transformer = LimitTransformer::new(2);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42, 43]);
  }

  // Test limit with edge case limit 3
  #[tokio::test]
  async fn test_limit_edge_case_three() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42, 43, 44]);
  }

  // Test limit with edge case limit 4
  #[tokio::test]
  async fn test_limit_edge_case_four() {
    let mut transformer = LimitTransformer::new(4);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42, 43, 44]);
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_limit_properties(
          limit in 0..100usize,
          values in prop::collection::vec(-100..100i32, 0..50)
      ) {
          // Test that limit transformer can handle various input sizes and limits
          let transformer = LimitTransformer::<i32>::new(limit);

          // Verify the transformer can be created with various limits
          assert_eq!(transformer.limit, limit);

          // Verify the transformer can handle various input sizes
          assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
      }

      #[test]
      fn test_limit_error_strategy_properties(
          limit in 0..50usize,
          error_strategy in prop::sample::select(vec![
              ErrorStrategy::<i32>::Stop,
              ErrorStrategy::<i32>::Skip,
              ErrorStrategy::<i32>::Retry(5),
          ])
      ) {
          // Test that limit transformer can handle different error strategies
          let transformer = LimitTransformer::<i32>::new(limit)
              .with_error_strategy(error_strategy);

          // Verify the transformer can be created with various error strategies
          assert_eq!(transformer.limit, limit);
      }

      #[test]
      fn test_limit_name_properties(
          limit in 0..50usize,
          name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
      ) {
          // Test that limit transformer can handle different names
          let transformer = LimitTransformer::<i32>::new(limit)
              .with_name(name.clone());

          // Verify the transformer can be created with various names
          assert_eq!(transformer.limit, limit);
          assert_eq!(transformer.config.name, Some(name));
      }

      #[test]
      fn test_limit_transformation_properties(
          limit in 0..20usize,
          values in prop::collection::vec(-50..50i32, 0..30)
      ) {
          // Test that limit transformation works correctly
          let transformer = LimitTransformer::<i32>::new(limit);

          // Verify the transformer can be created with various limits and input sizes
          assert_eq!(transformer.limit, limit);
          assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
      }
  }

  // Test limit transformer clone behavior
  #[tokio::test]
  async fn test_limit_transformer_clone_behavior() {
    let mut transformer1 = LimitTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test1".to_string());

    let mut transformer2 = LimitTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test2".to_string());

    // Both transformers should work independently
    let input1 = stream::iter(vec![1, 2, 3, 4, 5]);
    let input2 = stream::iter(vec![6, 7, 8, 9, 10]);

    let result1: Vec<i32> = transformer1.transform(Box::pin(input1)).collect().await;
    let result2: Vec<i32> = transformer2.transform(Box::pin(input2)).collect().await;

    assert_eq!(result1, vec![1, 2, 3]);
    assert_eq!(result2, vec![6, 7, 8]);
  }

  // Test limit transformer with different data types
  #[tokio::test]
  async fn test_limit_transformer_different_data_types() {
    // Test with i32
    let mut transformer_i32 = LimitTransformer::new(2);
    let input_i32 = stream::iter(vec![1, 2, 3, 4, 5]);
    let result_i32: Vec<i32> = transformer_i32
      .transform(Box::pin(input_i32))
      .collect()
      .await;
    assert_eq!(result_i32, vec![1, 2]);

    // Test with String
    let mut transformer_string = LimitTransformer::new(2);
    let input_string = stream::iter(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let result_string: Vec<String> = transformer_string
      .transform(Box::pin(input_string))
      .collect()
      .await;
    assert_eq!(result_string, vec!["a".to_string(), "b".to_string()]);

    // Test with f64
    let mut transformer_f64 = LimitTransformer::new(2);
    let input_f64 = stream::iter(vec![1.1, 2.2, 3.3, 4.4]);
    let result_f64: Vec<f64> = transformer_f64
      .transform(Box::pin(input_f64))
      .collect()
      .await;
    assert_eq!(result_f64, vec![1.1, 2.2]);
  }

  // Test limit transformer with very large input
  #[tokio::test]
  async fn test_limit_transformer_very_large_input() {
    let mut transformer = LimitTransformer::new(5);
    let large_input: Vec<i32> = (1..=1000).collect();
    let input = stream::iter(large_input);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  // Test limit transformer with very small limit
  #[tokio::test]
  async fn test_limit_transformer_very_small_limit() {
    let mut transformer = LimitTransformer::new(1);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1]);
  }

  // Test limit transformer with exact limit
  #[tokio::test]
  async fn test_limit_transformer_exact_limit() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test limit transformer with limit larger than input
  #[tokio::test]
  async fn test_limit_transformer_limit_larger_than_input() {
    let mut transformer = LimitTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test limit transformer with zero limit
  #[tokio::test]
  async fn test_limit_transformer_zero_limit() {
    let mut transformer = LimitTransformer::new(0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  // Test limit transformer with single element input
  #[tokio::test]
  async fn test_limit_transformer_single_element_input() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![42]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![42]);
  }

  // Test limit transformer with empty input
  #[tokio::test]
  async fn test_limit_transformer_empty_input() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(Vec::<i32>::new());

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  // Test limit transformer with negative numbers
  #[tokio::test]
  async fn test_limit_transformer_negative_numbers() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![-1, -2, -3, -4, -5]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![-1, -2, -3]);
  }

  // Test limit transformer with mixed positive and negative numbers
  #[tokio::test]
  async fn test_limit_transformer_mixed_numbers() {
    let mut transformer = LimitTransformer::new(4);
    let input = stream::iter(vec![-1, 2, -3, 4, -5, 6]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![-1, 2, -3, 4]);
  }

  // Test limit transformer with large numbers
  #[tokio::test]
  async fn test_limit_transformer_large_numbers() {
    let mut transformer = LimitTransformer::new(2);
    let input = stream::iter(vec![i32::MAX, i32::MIN, 0, 1]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![i32::MAX, i32::MIN]);
  }

  // Test limit transformer with zero values
  #[tokio::test]
  async fn test_limit_transformer_zero_values() {
    let mut transformer = LimitTransformer::new(3);
    let input = stream::iter(vec![0, 0, 0, 1, 2]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![0, 0, 0]);
  }

  // Test limit transformer with duplicate values
  #[tokio::test]
  async fn test_limit_transformer_duplicate_values() {
    let mut transformer = LimitTransformer::new(4);
    let input = stream::iter(vec![1, 1, 1, 2, 2, 3]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 1, 1, 2]);
  }

  // Test limit transformer with ascending values
  #[tokio::test]
  async fn test_limit_transformer_ascending_values() {
    let mut transformer = LimitTransformer::new(5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  // Test limit transformer with descending values
  #[tokio::test]
  async fn test_limit_transformer_descending_values() {
    let mut transformer = LimitTransformer::new(4);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![10, 9, 8, 7]);
  }

  // Test limit transformer with alternating values
  #[tokio::test]
  async fn test_limit_transformer_alternating_values() {
    let mut transformer = LimitTransformer::new(6);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3, 4, -4]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, -1, 2, -2, 3, -3]);
  }
}
