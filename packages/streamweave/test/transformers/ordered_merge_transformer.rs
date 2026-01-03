//! Tests for OrderedMergeTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::{MergeStrategy, OrderedMergeTransformer};
use streamweave::{Transformer, TransformerConfig};

#[test]
fn test_merge_strategy_variants() {
  assert_eq!(MergeStrategy::Sequential, MergeStrategy::Sequential);
  assert_eq!(MergeStrategy::RoundRobin, MergeStrategy::RoundRobin);
  assert_eq!(MergeStrategy::Priority, MergeStrategy::Priority);
  assert_eq!(MergeStrategy::Interleave, MergeStrategy::Interleave);
  assert_eq!(MergeStrategy::default(), MergeStrategy::Interleave);
}

#[tokio::test]
async fn test_ordered_merge_transformer_basic() {
  let mut transformer = OrderedMergeTransformer::<i32>::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_ordered_merge_transformer_with_strategy() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

  assert_eq!(transformer.strategy, MergeStrategy::RoundRobin);
}

#[tokio::test]
async fn test_ordered_merge_transformer_empty() {
  let mut transformer = OrderedMergeTransformer::<i32>::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_ordered_merge_transformer_with_name() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_name("ordered_merge".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("ordered_merge"));
}

#[tokio::test]
async fn test_ordered_merge_transformer_with_error_strategy() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_ordered_merge_transformer_clone() {
  let transformer1 = OrderedMergeTransformer::<i32>::new();
  let transformer2 = transformer1.clone();

  assert_eq!(transformer2.strategy, MergeStrategy::Interleave);
}
