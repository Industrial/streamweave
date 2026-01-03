//! Tests for ZipTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::ZipTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_zip_transformer_basic() {
  let mut transformer = ZipTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![vec![1, 4], vec![2, 5], vec![3, 6]]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Zip should transpose: [[1,4], [2,5], [3,6]] -> [[1,2,3], [4,5,6]]
  assert_eq!(results.len(), 2);
  assert_eq!(results[0], vec![1, 2, 3]);
  assert_eq!(results[1], vec![4, 5, 6]);
}

#[tokio::test]
async fn test_zip_transformer_unequal_lengths() {
  let mut transformer = ZipTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![vec![1, 4, 7], vec![2, 5], vec![3]]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should zip up to the shortest length
  assert_eq!(results.len(), 1);
  assert_eq!(results[0], vec![1, 2, 3]);
}

#[tokio::test]
async fn test_zip_transformer_empty() {
  let mut transformer = ZipTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<Vec<i32>>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_zip_transformer_single_vector() {
  let mut transformer = ZipTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![vec![1, 2, 3]]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], vec![1]);
  assert_eq!(results[1], vec![2]);
  assert_eq!(results[2], vec![3]);
}

#[tokio::test]
async fn test_zip_transformer_default() {
  let transformer = ZipTransformer::<i32>::default();
  let input_stream = Box::pin(stream::iter(vec![vec![1, 2]]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_zip_transformer_with_name() {
  let transformer = ZipTransformer::new().with_name("zipper".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("zipper"));
}

#[tokio::test]
async fn test_zip_transformer_with_error_strategy() {
  let transformer = ZipTransformer::new().with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}
