//! Tests for RoundRobinTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::{RoundRobinConfig, RoundRobinTransformer};
use streamweave::{Transformer, TransformerConfig};

#[test]
fn test_round_robin_config_default() {
  let config = RoundRobinConfig::default();
  assert_eq!(config.num_consumers, 2);
  assert_eq!(config.include_index, false);
}

#[test]
fn test_round_robin_config_new() {
  let config = RoundRobinConfig::new(5);
  assert_eq!(config.num_consumers, 5);
  assert_eq!(config.include_index, false);
}

#[test]
fn test_round_robin_config_with_include_index() {
  let config = RoundRobinConfig::new(3).with_include_index(true);
  assert_eq!(config.num_consumers, 3);
  assert_eq!(config.include_index, true);
}

#[tokio::test]
async fn test_round_robin_transformer_basic() {
  let mut transformer = RoundRobinTransformer::new(2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should distribute items round-robin
  assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_round_robin_transformer_empty() {
  let mut transformer = RoundRobinTransformer::new(2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_round_robin_transformer_with_name() {
  let transformer = RoundRobinTransformer::new(2).with_name("round_robin".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("round_robin"));
}

#[tokio::test]
async fn test_round_robin_transformer_with_error_strategy() {
  let transformer = RoundRobinTransformer::new(2).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_round_robin_transformer_clone() {
  let transformer1 = RoundRobinTransformer::new(3);
  let transformer2 = transformer1.clone();

  assert_eq!(transformer2.rr_config.num_consumers, 3);
}
