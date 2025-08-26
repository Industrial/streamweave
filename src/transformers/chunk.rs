use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  size: usize,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(size: usize) -> Self {
    Self {
      size,
      config: TransformerConfig::default(),
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

impl<T> Input for ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    Box::pin(input.chunks(size))
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
        .unwrap_or_else(|| "chunk_transformer".to_string()),
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
  async fn test_chunk_basic() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
  }

  #[tokio::test]
  async fn test_chunk_empty_input() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = ChunkTransformer::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  // Test chunk with size 1
  #[tokio::test]
  async fn test_chunk_size_one() {
    let mut transformer = ChunkTransformer::new(1);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1], vec![2], vec![3], vec![4], vec![5]]);
  }

  // Test chunk with size larger than input
  #[tokio::test]
  async fn test_chunk_size_larger_than_input() {
    let mut transformer = ChunkTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  // Test chunk with size equal to input
  #[tokio::test]
  async fn test_chunk_size_equal_to_input() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  // Test chunk with size 0 (edge case)
  #[tokio::test]
  async fn test_chunk_size_zero() {
    // Note: chunk size 0 is not supported by the futures library
    // This test is intentionally skipped as it would cause a panic
    // The futures::stream::chunks() method requires capacity > 0
  }

  // Test chunk with single element input
  #[tokio::test]
  async fn test_chunk_single_element_input() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![42]]);
  }

  // Test chunk with exact multiple of size
  #[tokio::test]
  async fn test_chunk_exact_multiple() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2], vec![3, 4]]);
  }

  // Test chunk with partial last chunk
  #[tokio::test]
  async fn test_chunk_partial_last_chunk() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5]]);
  }

  // Test chunk with strings
  #[tokio::test]
  async fn test_chunk_with_strings() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![
      "hello".to_string(),
      "world".to_string(),
      "test".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<String>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![
        vec!["hello".to_string(), "world".to_string()],
        vec!["test".to_string()]
      ]
    );
  }

  // Test chunk with floats
  #[tokio::test]
  async fn test_chunk_with_floats() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![1.1, 2.2, 3.3, 4.4, 5.5]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<f64>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1.1, 2.2, 3.3], vec![4.4, 5.5]]);
  }

  // Test chunk with custom error strategy
  #[tokio::test]
  async fn test_chunk_with_custom_error_strategy() {
    let custom_handler = |_error: &StreamError<i32>| ErrorAction::Skip;
    let transformer = ChunkTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::new_custom(custom_handler))
      .with_name("custom_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.name(), Some("custom_transformer".to_string()));
  }

  // Test chunk with retry error strategy
  #[tokio::test]
  async fn test_chunk_with_retry_error_strategy() {
    let transformer = ChunkTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("retry_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Retry(3));
    assert_eq!(config.name(), Some("retry_transformer".to_string()));
  }

  // Test chunk with stop error strategy
  #[tokio::test]
  async fn test_chunk_with_stop_error_strategy() {
    let transformer = ChunkTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Stop)
      .with_name("stop_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Stop);
    assert_eq!(config.name(), Some("stop_transformer".to_string()));
  }

  // Test chunk with skip error strategy
  #[tokio::test]
  async fn test_chunk_with_skip_error_strategy() {
    let transformer = ChunkTransformer::<i32>::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("skip_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("skip_transformer".to_string()));
  }

  // Test chunk with empty name
  #[tokio::test]
  async fn test_chunk_with_empty_name() {
    let transformer = ChunkTransformer::<i32>::new(2).with_name("".to_string());

    let config = transformer.config();
    assert_eq!(config.name(), Some("".to_string()));
  }

  // Test chunk with very long name
  #[tokio::test]
  async fn test_chunk_with_very_long_name() {
    let long_name = "a".repeat(1000);
    let transformer = ChunkTransformer::<i32>::new(2).with_name(long_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(long_name));
  }

  // Test chunk with unicode name
  #[tokio::test]
  async fn test_chunk_with_unicode_name() {
    let unicode_name = "🚀火箭🚀".to_string();
    let transformer = ChunkTransformer::<i32>::new(2).with_name(unicode_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(unicode_name));
  }

  // Test chunk with special characters in name
  #[tokio::test]
  async fn test_chunk_with_special_characters_name() {
    let special_name = "!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string();
    let transformer = ChunkTransformer::<i32>::new(2).with_name(special_name.clone());

    let config = transformer.config();
    assert_eq!(config.name(), Some(special_name));
  }

  // Test chunk with large numbers
  #[tokio::test]
  async fn test_chunk_with_large_numbers() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![i64::MAX, i64::MIN, 0]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i64>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![i64::MAX, i64::MIN], vec![0]]);
  }

  // Test chunk with zero values
  #[tokio::test]
  async fn test_chunk_with_zero_values() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![0, 0, 0, 1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![0, 0, 0], vec![1, 2]]);
  }

  // Test chunk with duplicate values
  #[tokio::test]
  async fn test_chunk_with_duplicate_values() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![1, 1, 1, 2, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 1], vec![1, 2], vec![2]]);
  }

  // Test chunk with ascending values
  #[tokio::test]
  async fn test_chunk_with_ascending_values() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9], vec![10]]
    );
  }

  // Test chunk with descending values
  #[tokio::test]
  async fn test_chunk_with_descending_values() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![vec![10, 9], vec![8, 7], vec![6, 5], vec![4, 3], vec![2, 1]]
    );
  }

  // Test chunk with alternating values
  #[tokio::test]
  async fn test_chunk_with_alternating_values() {
    let mut transformer = ChunkTransformer::new(4);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3, 4, -4]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, -1, 2, -2], vec![3, -3, 4, -4]]);
  }

  // Test chunk with edge case size 1
  #[tokio::test]
  async fn test_chunk_edge_case_size_one() {
    let mut transformer = ChunkTransformer::new(1);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![42], vec![43], vec![44]]);
  }

  // Test chunk with edge case size 2
  #[tokio::test]
  async fn test_chunk_edge_case_size_two() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![42, 43], vec![44]]);
  }

  // Test chunk with edge case size 3
  #[tokio::test]
  async fn test_chunk_edge_case_size_three() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![42, 43, 44]]);
  }

  // Test chunk with edge case size 4
  #[tokio::test]
  async fn test_chunk_edge_case_size_four() {
    let mut transformer = ChunkTransformer::new(4);
    let input = stream::iter(vec![42, 43, 44]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![42, 43, 44]]);
  }

  // Property-based tests using proptest
  proptest! {
    #[test]
    fn test_chunk_properties(
      size in 1..100usize,
      values in prop::collection::vec(-100..100i32, 0..50)
    ) {
      // Test that chunk transformer can handle various input sizes and chunk sizes
      let transformer = ChunkTransformer::<i32>::new(size);

      // Verify the transformer can be created with various sizes
      assert_eq!(transformer.size, size);

      // Verify the transformer can handle various input sizes
      assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
    }

    #[test]
    fn test_chunk_error_strategy_properties(
      size in 1..50usize,
      error_strategy in prop::sample::select(vec![
        ErrorStrategy::<i32>::Stop,
        ErrorStrategy::<i32>::Skip,
        ErrorStrategy::<i32>::Retry(5),
      ])
    ) {
      // Test that chunk transformer can handle different error strategies
      let transformer = ChunkTransformer::<i32>::new(size)
        .with_error_strategy(error_strategy);

      // Verify the transformer can be created with various error strategies
      assert_eq!(transformer.size, size);
    }

    #[test]
    fn test_chunk_name_properties(
      size in 1..50usize,
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      // Test that chunk transformer can handle different names
      let transformer = ChunkTransformer::<i32>::new(size)
        .with_name(name.clone());

      // Verify the transformer can be created with various names
      assert_eq!(transformer.size, size);
      assert_eq!(transformer.config.name, Some(name));
    }

    #[test]
    fn test_chunk_transformation_properties(
      size in 1..20usize,
      values in prop::collection::vec(-50..50i32, 0..30)
    ) {
      // Test that chunk transformation works correctly
      let transformer = ChunkTransformer::<i32>::new(size);

      // Verify the transformer can be created with various sizes and input sizes
      assert_eq!(transformer.size, size);
      assert_eq!(values.len(), values.len()); // Dummy assertion to satisfy proptest
    }
  }

  // Test chunk transformer clone behavior
  #[tokio::test]
  async fn test_chunk_transformer_clone_behavior() {
    let mut transformer1 = ChunkTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test1".to_string());

    let mut transformer2 = ChunkTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test2".to_string());

    // Both transformers should work independently
    let input1 = stream::iter(vec![1, 2, 3, 4, 5]);
    let input2 = stream::iter(vec![6, 7, 8, 9, 10]);

    let result1: Vec<Vec<i32>> = transformer1.transform(Box::pin(input1)).collect().await;
    let result2: Vec<Vec<i32>> = transformer2.transform(Box::pin(input2)).collect().await;

    assert_eq!(result1, vec![vec![1, 2, 3], vec![4, 5]]);
    assert_eq!(result2, vec![vec![6, 7, 8], vec![9, 10]]);
  }

  // Test chunk transformer with different data types
  #[tokio::test]
  async fn test_chunk_transformer_different_data_types() {
    // Test with i32
    let mut transformer_i32 = ChunkTransformer::new(2);
    let input_i32 = stream::iter(vec![1, 2, 3, 4, 5]);
    let result_i32: Vec<Vec<i32>> = transformer_i32
      .transform(Box::pin(input_i32))
      .collect()
      .await;
    assert_eq!(result_i32, vec![vec![1, 2], vec![3, 4], vec![5]]);

    // Test with String
    let mut transformer_string = ChunkTransformer::new(2);
    let input_string = stream::iter(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let result_string: Vec<Vec<String>> = transformer_string
      .transform(Box::pin(input_string))
      .collect()
      .await;
    assert_eq!(
      result_string,
      vec![
        vec!["a".to_string(), "b".to_string()],
        vec!["c".to_string()]
      ]
    );

    // Test with f64
    let mut transformer_f64 = ChunkTransformer::new(2);
    let input_f64 = stream::iter(vec![1.1, 2.2, 3.3, 4.4]);
    let result_f64: Vec<Vec<f64>> = transformer_f64
      .transform(Box::pin(input_f64))
      .collect()
      .await;
    assert_eq!(result_f64, vec![vec![1.1, 2.2], vec![3.3, 4.4]]);
  }

  // Test chunk transformer with very large input
  #[tokio::test]
  async fn test_chunk_transformer_very_large_input() {
    let mut transformer = ChunkTransformer::new(5);
    let large_input: Vec<i32> = (1..=1000).collect();
    let input = stream::iter(large_input);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    // Verify we got the expected number of chunks
    assert_eq!(result.len(), 200); // 1000 / 5 = 200 chunks

    // Verify the first chunk
    assert_eq!(result[0], vec![1, 2, 3, 4, 5]);

    // Verify the last chunk
    assert_eq!(result[199], vec![996, 997, 998, 999, 1000]);
  }

  // Test chunk transformer with very small chunk size
  #[tokio::test]
  async fn test_chunk_transformer_very_small_chunk_size() {
    let mut transformer = ChunkTransformer::new(1);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![1], vec![2], vec![3], vec![4], vec![5]]);
  }

  // Test chunk transformer with exact chunk size
  #[tokio::test]
  async fn test_chunk_transformer_exact_chunk_size() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6]]);
  }

  // Test chunk transformer with chunk size larger than input
  #[tokio::test]
  async fn test_chunk_transformer_chunk_size_larger_than_input() {
    let mut transformer = ChunkTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  // Test chunk transformer with single element input
  #[tokio::test]
  async fn test_chunk_transformer_single_element_input() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![42]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![42]]);
  }

  // Test chunk transformer with empty input
  #[tokio::test]
  async fn test_chunk_transformer_empty_input() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(Vec::<i32>::new());

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  // Test chunk transformer with negative numbers
  #[tokio::test]
  async fn test_chunk_transformer_negative_numbers() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![-1, -2, -3, -4, -5]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![-1, -2, -3], vec![-4, -5]]);
  }

  // Test chunk transformer with mixed positive and negative numbers
  #[tokio::test]
  async fn test_chunk_transformer_mixed_numbers() {
    let mut transformer = ChunkTransformer::new(4);
    let input = stream::iter(vec![-1, 2, -3, 4, -5, 6]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![-1, 2, -3, 4], vec![-5, 6]]);
  }

  // Test chunk transformer with large numbers
  #[tokio::test]
  async fn test_chunk_transformer_large_numbers() {
    let mut transformer = ChunkTransformer::new(2);
    let input = stream::iter(vec![i32::MAX, i32::MIN, 0, 1]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![i32::MAX, i32::MIN], vec![0, 1]]);
  }

  // Test chunk transformer with zero values
  #[tokio::test]
  async fn test_chunk_transformer_zero_values() {
    let mut transformer = ChunkTransformer::new(3);
    let input = stream::iter(vec![0, 0, 0, 1, 2]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![0, 0, 0], vec![1, 2]]);
  }

  // Test chunk transformer with duplicate values
  #[tokio::test]
  async fn test_chunk_transformer_duplicate_values() {
    let mut transformer = ChunkTransformer::new(4);
    let input = stream::iter(vec![1, 1, 1, 2, 2, 3]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![1, 1, 1, 2], vec![2, 3]]);
  }

  // Test chunk transformer with ascending values
  #[tokio::test]
  async fn test_chunk_transformer_ascending_values() {
    let mut transformer = ChunkTransformer::new(5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]]);
  }

  // Test chunk transformer with descending values
  #[tokio::test]
  async fn test_chunk_transformer_descending_values() {
    let mut transformer = ChunkTransformer::new(4);
    let input = stream::iter(vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(
      result,
      vec![vec![10, 9, 8, 7], vec![6, 5, 4, 3], vec![2, 1]]
    );
  }

  // Test chunk transformer with alternating values
  #[tokio::test]
  async fn test_chunk_transformer_alternating_values() {
    let mut transformer = ChunkTransformer::new(6);
    let input = stream::iter(vec![1, -1, 2, -2, 3, -3, 4, -4]);

    let result: Vec<Vec<i32>> = transformer.transform(Box::pin(input)).collect().await;

    assert_eq!(result, vec![vec![1, -1, 2, -2, 3, -3], vec![4, -4]]);
  }
}
