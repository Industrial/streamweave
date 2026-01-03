//! Tests for ReduceTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::ReduceTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_reduce_transformer_sum() {
  let mut transformer = ReduceTransformer::new(0, |acc: i32, x: i32| acc + x);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Reduce should emit intermediate results
  assert!(!results.is_empty());
  // Last result should be the sum
  assert_eq!(results.last(), Some(&15));
}

#[tokio::test]
async fn test_reduce_transformer_product() {
  let mut transformer = ReduceTransformer::new(1, |acc: i32, x: i32| acc * x);
  let input_stream = Box::pin(stream::iter(vec![2, 3, 4]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert!(!results.is_empty());
  assert_eq!(results.last(), Some(&24)); // 1 * 2 * 3 * 4 = 24
}

#[tokio::test]
async fn test_reduce_transformer_empty() {
  let mut transformer = ReduceTransformer::new(0, |acc: i32, x: i32| acc + x);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  // Should emit initial accumulator value
  let result = output_stream.next().await;
  assert_eq!(result, Some(0));
}

#[tokio::test]
async fn test_reduce_transformer_with_name() {
  let transformer =
    ReduceTransformer::new(0, |acc: i32, x: i32| acc + x).with_name("sum".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("sum"));
}

#[tokio::test]
async fn test_reduce_transformer_with_error_strategy() {
  let transformer =
    ReduceTransformer::new(0, |acc: i32, x: i32| acc + x).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}
