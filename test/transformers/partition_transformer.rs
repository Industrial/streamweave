//! Tests for PartitionTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::PartitionTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_partition_transformer_basic() {
  let mut transformer = PartitionTransformer::new(|x: &i32| *x > 2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(partition) = output_stream.next().await {
    results.push(partition);
  }

  // Should have one partition result
  assert_eq!(results.len(), 1);
  let (matches, non_matches) = &results[0];
  assert_eq!(matches.len(), 3); // 3, 4, 5
  assert_eq!(non_matches.len(), 2); // 1, 2
}

#[tokio::test]
async fn test_partition_transformer_all_match() {
  let mut transformer = PartitionTransformer::new(|x: &i32| *x > 0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(partition) = output_stream.next().await {
    results.push(partition);
  }

  assert_eq!(results.len(), 1);
  let (matches, non_matches) = &results[0];
  assert_eq!(matches.len(), 3);
  assert_eq!(non_matches.len(), 0);
}

#[tokio::test]
async fn test_partition_transformer_none_match() {
  let mut transformer = PartitionTransformer::new(|x: &i32| *x > 10);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(partition) = output_stream.next().await {
    results.push(partition);
  }

  assert_eq!(results.len(), 1);
  let (matches, non_matches) = &results[0];
  assert_eq!(matches.len(), 0);
  assert_eq!(non_matches.len(), 3);
}

#[tokio::test]
async fn test_partition_transformer_empty() {
  let mut transformer = PartitionTransformer::new(|x: &i32| *x > 2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_partition_transformer_with_name() {
  let transformer =
    PartitionTransformer::new(|x: &i32| *x > 2).with_name("partitioner".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("partitioner"));
}

#[tokio::test]
async fn test_partition_transformer_with_error_strategy() {
  let transformer =
    PartitionTransformer::new(|x: &i32| *x > 2).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}
