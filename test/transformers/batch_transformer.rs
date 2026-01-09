//! Tests for BatchTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::BatchTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_batch_transformer_basic() {
  let mut transformer = BatchTransformer::new(3).unwrap();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6, 7]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(batch) = output_stream.next().await {
    results.push(batch);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], vec![1, 2, 3]);
  assert_eq!(results[1], vec![4, 5, 6]);
  assert_eq!(results[2], vec![7]); // Partial batch
}

#[tokio::test]
async fn test_batch_transformer_exact_size() {
  let mut transformer = BatchTransformer::new(2).unwrap();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(batch) = output_stream.next().await {
    results.push(batch);
  }

  assert_eq!(results.len(), 2);
  assert_eq!(results[0], vec![1, 2]);
  assert_eq!(results[1], vec![3, 4]);
}

#[tokio::test]
async fn test_batch_transformer_single_batch() {
  let mut transformer = BatchTransformer::new(5).unwrap();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(batch) = output_stream.next().await {
    results.push(batch);
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], vec![1, 2, 3]);
}

#[tokio::test]
async fn test_batch_transformer_empty() {
  let mut transformer = BatchTransformer::new(3).unwrap();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[test]
fn test_batch_transformer_zero_size_error() {
  let result = BatchTransformer::<i32>::new(0);
  assert!(result.is_err());
}

#[test]
fn test_batch_transformer_with_name() {
  let transformer = BatchTransformer::new(3)
    .unwrap()
    .with_name("batch_3".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("batch_3"));
}

#[test]
fn test_batch_transformer_with_error_strategy() {
  let transformer = BatchTransformer::new(3)
    .unwrap()
    .with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_batch_transformer_component_info() {
  let transformer = BatchTransformer::new(3)
    .unwrap()
    .with_name("test".to_string());

  let info = transformer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("BatchTransformer"));
}
