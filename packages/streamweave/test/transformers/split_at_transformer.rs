//! Tests for SplitAtTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::SplitAtTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_split_at_transformer_basic() {
  let mut transformer = SplitAtTransformer::new(2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(partition) = output_stream.next().await {
    results.push(partition);
  }

  // Should split at index 2
  assert_eq!(results.len(), 1);
  let (before, after) = &results[0];
  assert_eq!(before.len(), 2); // [1, 2]
  assert_eq!(after.len(), 3); // [3, 4, 5]
}

#[tokio::test]
async fn test_split_at_transformer_zero() {
  let mut transformer = SplitAtTransformer::new(0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(partition) = output_stream.next().await {
    results.push(partition);
  }

  assert_eq!(results.len(), 1);
  let (before, after) = &results[0];
  assert_eq!(before.len(), 0);
  assert_eq!(after.len(), 3);
}

#[tokio::test]
async fn test_split_at_transformer_beyond_length() {
  let mut transformer = SplitAtTransformer::new(10);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(partition) = output_stream.next().await {
    results.push(partition);
  }

  assert_eq!(results.len(), 1);
  let (before, after) = &results[0];
  assert_eq!(before.len(), 3);
  assert_eq!(after.len(), 0);
}

#[tokio::test]
async fn test_split_at_transformer_empty() {
  let mut transformer = SplitAtTransformer::new(2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_split_at_transformer_with_name() {
  let transformer = SplitAtTransformer::new(2).with_name("split_at".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("split_at"));
}

#[tokio::test]
async fn test_split_at_transformer_with_error_strategy() {
  let transformer = SplitAtTransformer::new(2).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_split_at_transformer_clone() {
  let transformer1 = SplitAtTransformer::new(2);
  let transformer2 = transformer1.clone();

  assert_eq!(transformer2.index, 2);
}
