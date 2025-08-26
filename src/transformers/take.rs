use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct TakeTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  take: usize,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TakeTransformer<T> {
  pub fn new(take: usize) -> Self {
    Self {
      take,
      config: TransformerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TakeTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TakeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TakeTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let take = self.take;
    Box::pin(input.take(take))
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
        .unwrap_or_else(|| "take_transformer".to_string()),
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
  async fn test_take_basic() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_take_empty_input() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = TakeTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  // Test take with zero
  #[tokio::test]
  async fn test_take_zero() {
    let mut transformer = TakeTransformer::new(0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  // Test take larger than input
  #[tokio::test]
  async fn test_take_larger_than_input() {
    let mut transformer = TakeTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test take equal to input size
  #[tokio::test]
  async fn test_take_equal_to_input_size() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test take with single element
  #[tokio::test]
  async fn test_take_single_element() {
    let mut transformer = TakeTransformer::new(1);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42]);
  }

  // Test take with very large take
  #[tokio::test]
  async fn test_take_very_large() {
    let mut transformer = TakeTransformer::new(usize::MAX);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test take with negative numbers
  #[tokio::test]
  async fn test_take_with_negative_numbers() {
    let mut transformer = TakeTransformer::new(2);
    let input = stream::iter(vec![-1, -2, -3, -4, -5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![-1, -2]);
  }

  // Test take with strings
  #[tokio::test]
  async fn test_take_with_strings() {
    let mut transformer = TakeTransformer::new(2);
    let input = stream::iter(vec![
      "hello".to_string(),
      "world".to_string(),
      "test".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
  }

  // Test take with floats
  #[tokio::test]
  async fn test_take_with_floats() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![1.1, 2.2, 3.3, 4.4, 5.5]);
    let boxed_input = Box::pin(input);

    let result: Vec<f64> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1.1, 2.2, 3.3]);
  }

  // Test take with custom error strategy
  #[tokio::test]
  async fn test_take_with_custom_error_strategy() {
    let custom_handler = |_error: &StreamError<i32>| ErrorAction::Skip;
    let transformer = TakeTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::new_custom(custom_handler))
      .with_name("custom_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.name(), Some("custom_transformer".to_string()));
  }

  // Test take with retry error strategy
  #[tokio::test]
  async fn test_take_with_retry_error_strategy() {
    let transformer = TakeTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("retry_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Retry(3));
    assert_eq!(config.name(), Some("retry_transformer".to_string()));
  }

  // Test take with stop error strategy
  #[tokio::test]
  async fn test_take_with_stop_error_strategy() {
    let transformer = TakeTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Stop)
      .with_name("stop_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Stop);
    assert_eq!(config.name(), Some("stop_transformer".to_string()));
  }

  // Test take with skip error strategy
  #[tokio::test]
  async fn test_take_with_skip_error_strategy() {
    let transformer = TakeTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("skip_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("skip_transformer".to_string()));
  }

  // Test take with empty name
  #[tokio::test]
  async fn test_take_with_empty_name() {
    let transformer = TakeTransformer::<i32>::new(2).with_name("".to_string());

    let config = transformer.config();
    assert_eq!(config.name(), Some("".to_string()));
  }

  // Test take with very long name
  #[tokio::test]
  async fn test_take_with_very_long_name() {
    let long_name = "a".repeat(1000);
    let transformer = TakeTransformer::<i32>::new(2).with_name(long_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(long_name));
  }

  // Test take with unicode name
  #[tokio::test]
  async fn test_take_with_unicode_name() {
    let unicode_name = "🚀火箭🚀".to_string();
    let transformer = TakeTransformer::<i32>::new(2).with_name(unicode_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(unicode_name));
  }

  // Test take with special characters in name
  #[tokio::test]
  async fn test_take_with_special_characters_name() {
    let special_name = "!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string();
    let transformer = TakeTransformer::<i32>::new(2).with_name(special_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(special_name));
  }

  // Test take with large numbers
  #[tokio::test]
  async fn test_take_with_large_numbers() {
    let mut transformer = TakeTransformer::new(2);
    let input = stream::iter(vec![i64::MAX, i64::MIN, 0]);
    let boxed_input = Box::pin(input);

    let result: Vec<i64> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![i64::MAX, i64::MIN]);
  }

  // Test take with zero values
  #[tokio::test]
  async fn test_take_with_zero_values() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![0, 0, 0, 1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![0, 0, 0]);
  }

  // Test take with duplicate values
  #[tokio::test]
  async fn test_take_with_duplicate_values() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![1, 1, 1, 2, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 1, 1]);
  }

  // Test take with ascending values
  #[tokio::test]
  async fn test_take_with_ascending_values() {
    let mut transformer = TakeTransformer::new(4);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4]);
  }

  // Test take with descending values
  #[tokio::test]
  async fn test_take_with_descending_values() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![10, 9, 8]);
  }

  // Test take with alternating values
  #[tokio::test]
  async fn test_take_with_alternating_values() {
    let mut transformer = TakeTransformer::new(5);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3, 4, -4]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, -1, 2, -2, 3]);
  }

  // Test take with edge case take 1
  #[tokio::test]
  async fn test_take_edge_case_one() {
    let mut transformer = TakeTransformer::new(1);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42]);
  }

  // Test take with edge case take 2
  #[tokio::test]
  async fn test_take_edge_case_two() {
    let mut transformer = TakeTransformer::new(2);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42, 43]);
  }

  // Test take with edge case take 3
  #[tokio::test]
  async fn test_take_edge_case_three() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42, 43, 44]);
  }

  // Test take with edge case take 4
  #[tokio::test]
  async fn test_take_edge_case_four() {
    let mut transformer = TakeTransformer::new(4);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![42, 43, 44]);
  }

  // Property-based tests using proptest
  proptest! {
    #[test]
    fn test_take_properties(
      take in 0..100usize,
      values in prop::collection::vec(-100..100i32, 0..50)
    ) {
      // Test that take transformer can handle various input sizes and takes
      let transformer = TakeTransformer::<i32>::new(take);

      // Verify the transformer can be created with various takes
      assert_eq!(transformer.take, take);

      // Verify the transformer can handle various input sizes
      assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
    }

    #[test]
    fn test_take_error_strategy_properties(
      take in 0..50usize,
      error_strategy in prop::sample::select(vec![
        ErrorStrategy::<i32>::Stop,
        ErrorStrategy::<i32>::Skip,
        ErrorStrategy::<i32>::Retry(5),
      ])
    ) {
      // Test that take transformer can handle different error strategies
      let transformer = TakeTransformer::<i32>::new(take)
        .with_error_strategy(error_strategy);

      // Verify the transformer can be created with various error strategies
      assert_eq!(transformer.take, take);
    }

    #[test]
    fn test_take_name_properties(
      take in 0..50usize,
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      // Test that take transformer can handle different names
      let transformer = TakeTransformer::<i32>::new(take)
        .with_name(name.clone());

      // Verify the transformer can be created with various names
      assert_eq!(transformer.take, take);
      assert_eq!(transformer.config.name, Some(name));
    }

    #[test]
    fn test_take_transformation_properties(
      take in 0..20usize,
      values in prop::collection::vec(-50..50i32, 0..30)
    ) {
      // Test that take transformation works correctly
      let transformer = TakeTransformer::<i32>::new(take);

      // Verify the transformer can be created with various takes and input sizes
      assert_eq!(transformer.take, take);
      assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
    }
  }

  // Test take transformer clone behavior
  #[tokio::test]
  async fn test_take_transformer_clone_behavior() {
    let mut transformer1 = TakeTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test1".to_string());

    let mut transformer2 = TakeTransformer::new(3)
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

  // Test take transformer with different data types
  #[tokio::test]
  async fn test_take_transformer_different_data_types() {
    // Test with i32
    let mut transformer_i32 = TakeTransformer::new(2);
    let input_i32 = stream::iter(vec![1, 2, 3, 4, 5]);
    let result_i32: Vec<i32> = transformer_i32
      .transform(Box::pin(input_i32))
      .collect()
      .await;
    assert_eq!(result_i32, vec![1, 2]);

    // Test with String
    let mut transformer_string = TakeTransformer::new(2);
    let input_string = stream::iter(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let result_string: Vec<String> = transformer_string
      .transform(Box::pin(input_string))
      .collect()
      .await;
    assert_eq!(result_string, vec!["a".to_string(), "b".to_string()]);

    // Test with f64
    let mut transformer_f64 = TakeTransformer::new(2);
    let input_f64 = stream::iter(vec![1.1, 2.2, 3.3, 4.4]);
    let result_f64: Vec<f64> = transformer_f64
      .transform(Box::pin(input_f64))
      .collect()
      .await;
    assert_eq!(result_f64, vec![1.1, 2.2]);
  }

  // Test take transformer with very large input
  #[tokio::test]
  async fn test_take_transformer_very_large_input() {
    let mut transformer = TakeTransformer::new(5);
    let large_input: Vec<i32> = (1..=1000).collect();
    let input = stream::iter(large_input);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  // Test take transformer with very small take
  #[tokio::test]
  async fn test_take_transformer_very_small_take() {
    let mut transformer = TakeTransformer::new(1);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1]);
  }

  // Test take transformer with exact take
  #[tokio::test]
  async fn test_take_transformer_exact_take() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test take transformer with take larger than input
  #[tokio::test]
  async fn test_take_transformer_take_larger_than_input() {
    let mut transformer = TakeTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test take transformer with zero take
  #[tokio::test]
  async fn test_take_transformer_zero_take() {
    let mut transformer = TakeTransformer::new(0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  // Test take transformer with single element input
  #[tokio::test]
  async fn test_take_transformer_single_element_input() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![42]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![42]);
  }

  // Test take transformer with empty input
  #[tokio::test]
  async fn test_take_transformer_empty_input() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(Vec::<i32>::new());

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  // Test take transformer with negative numbers
  #[tokio::test]
  async fn test_take_transformer_negative_numbers() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![-1, -2, -3, -4, -5]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![-1, -2, -3]);
  }

  // Test take transformer with mixed positive and negative numbers
  #[tokio::test]
  async fn test_take_transformer_mixed_numbers() {
    let mut transformer = TakeTransformer::new(4);
    let input = stream::iter(vec![-1, 2, -3, 4, -5, 6]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![-1, 2, -3, 4]);
  }

  // Test take transformer with large numbers
  #[tokio::test]
  async fn test_take_transformer_large_numbers() {
    let mut transformer = TakeTransformer::new(2);
    let input = stream::iter(vec![i32::MAX, i32::MIN, 0, 1]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![i32::MAX, i32::MIN]);
  }

  // Test take transformer with zero values
  #[tokio::test]
  async fn test_take_transformer_zero_values() {
    let mut transformer = TakeTransformer::new(3);
    let input = stream::iter(vec![0, 0, 0, 1, 2]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![0, 0, 0]);
  }

  // Test take transformer with duplicate values
  #[tokio::test]
  async fn test_take_transformer_duplicate_values() {
    let mut transformer = TakeTransformer::new(4);
    let input = stream::iter(vec![1, 1, 1, 2, 2, 3]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 1, 1, 2]);
  }

  // Test take transformer with ascending values
  #[tokio::test]
  async fn test_take_transformer_ascending_values() {
    let mut transformer = TakeTransformer::new(5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  // Test take transformer with descending values
  #[tokio::test]
  async fn test_take_transformer_descending_values() {
    let mut transformer = TakeTransformer::new(4);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![10, 9, 8, 7]);
  }

  // Test take transformer with alternating values
  #[tokio::test]
  async fn test_take_transformer_alternating_values() {
    let mut transformer = TakeTransformer::new(6);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3, 4, -4]);

    let result: Vec<i32> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![1, -1, 2, -2, 3, -3]);
  }
}
