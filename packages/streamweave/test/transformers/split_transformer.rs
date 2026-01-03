//! Tests for SplitTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::SplitTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_split_transformer_basic() {
  let mut transformer = SplitTransformer::new(|x: &i32| *x == 0);
  let input_stream = Box::pin(stream::iter(vec![vec![1, 2, 0, 3, 4, 0, 5]]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(chunk) = output_stream.next().await {
    results.push(chunk);
  }

  // Should split at zeros: [1,2], [3,4], [5]
  assert_eq!(results.len(), 3);
  assert_eq!(results[0], vec![1, 2]);
  assert_eq!(results[1], vec![3, 4]);
  assert_eq!(results[2], vec![5]);
}

#[tokio::test]
async fn test_split_transformer_no_splits() {
  let mut transformer = SplitTransformer::new(|x: &i32| *x == 0);
  let input_stream = Box::pin(stream::iter(vec![vec![1, 2, 3, 4]]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(chunk) = output_stream.next().await {
    results.push(chunk);
  }

  // Should return single chunk if no splits
  assert_eq!(results.len(), 1);
  assert_eq!(results[0], vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn test_split_transformer_empty() {
  let mut transformer = SplitTransformer::new(|x: &i32| *x == 0);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<Vec<i32>>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_split_transformer_with_name() {
  let transformer = SplitTransformer::new(|x: &i32| *x == 0).with_name("splitter".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("splitter"));
}

#[tokio::test]
async fn test_split_transformer_with_error_strategy() {
  let transformer =
    SplitTransformer::new(|x: &i32| *x == 0).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}
