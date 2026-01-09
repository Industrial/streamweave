//! Tests for GroupByTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::GroupByTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_group_by_transformer_basic() {
  let mut transformer = GroupByTransformer::new(|x: &i32| *x % 2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(group) = output_stream.next().await {
    results.push(group);
  }

  // Should group by even/odd
  assert_eq!(results.len(), 2);
  let mut keys: Vec<i32> = results.iter().map(|(k, _)| *k).collect();
  keys.sort();
  assert_eq!(keys, vec![0, 1]);
}

#[tokio::test]
async fn test_group_by_transformer_single_group() {
  let mut transformer = GroupByTransformer::new(|x: &i32| 0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(group) = output_stream.next().await {
    results.push(group);
  }

  // All items should be in one group
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].0, 0);
  assert_eq!(results[0].1.len(), 3);
}

#[tokio::test]
async fn test_group_by_transformer_empty() {
  let mut transformer = GroupByTransformer::new(|x: &i32| *x % 2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_group_by_transformer_with_name() {
  let transformer = GroupByTransformer::new(|x: &i32| *x % 2).with_name("grouper".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("grouper"));
}

#[tokio::test]
async fn test_group_by_transformer_with_error_strategy() {
  let transformer =
    GroupByTransformer::new(|x: &i32| *x % 2).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_group_by_transformer_clone() {
  let transformer1 = GroupByTransformer::new(|x: &i32| *x % 2);
  let transformer2 = transformer1.clone();

  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));
  let mut output_stream = transformer2.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(group) = output_stream.next().await {
    results.push(group);
  }

  assert!(!results.is_empty());
}
