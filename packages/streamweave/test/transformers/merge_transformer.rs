//! Tests for MergeTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::MergeTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_merge_transformer_basic() {
  let mut transformer = MergeTransformer::new();
  let stream1 = Box::pin(stream::iter(vec![1, 2, 3]));
  let stream2 = Box::pin(stream::iter(vec![4, 5, 6]));

  transformer.add_stream(stream2);
  let input_stream = stream1;

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Results should contain items from both streams (order may vary)
  assert_eq!(results.len(), 6);
  results.sort();
  assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
}

#[tokio::test]
async fn test_merge_transformer_empty() {
  let mut transformer = MergeTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_merge_transformer_default() {
  let transformer = MergeTransformer::<i32>::default();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_merge_transformer_with_name() {
  let transformer = MergeTransformer::new().with_name("merger".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("merger"));
}

#[tokio::test]
async fn test_merge_transformer_with_error_strategy() {
  let transformer = MergeTransformer::new().with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_merge_transformer_add_stream() {
  let mut transformer = MergeTransformer::new();
  let stream1 = Box::pin(stream::iter(vec![1, 2]));
  let stream2 = Box::pin(stream::iter(vec![3, 4]));

  transformer.add_stream(stream1);
  transformer.add_stream(stream2);

  let input_stream = Box::pin(stream::iter(vec![5, 6]));
  let mut output_stream = transformer.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 6);
  results.sort();
  assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
}
