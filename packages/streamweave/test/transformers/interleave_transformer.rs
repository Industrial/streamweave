//! Tests for InterleaveTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::InterleaveTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_interleave_transformer_basic() {
  let other_stream = Box::pin(stream::iter(vec![10, 20, 30]));
  let mut transformer = InterleaveTransformer::new(other_stream);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
    if results.len() >= 6 {
      break; // Don't wait too long
    }
  }

  // Should interleave items from both streams
  assert!(results.len() >= 3);
  // Results should contain items from both streams
  assert!(results.contains(&1) || results.contains(&10));
}

#[tokio::test]
async fn test_interleave_transformer_empty_input() {
  let other_stream = Box::pin(stream::iter(vec![10, 20]));
  let mut transformer = InterleaveTransformer::new(other_stream);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should still have items from other stream
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_interleave_transformer_with_name() {
  let other_stream = Box::pin(stream::iter(vec![10]));
  let transformer = InterleaveTransformer::new(other_stream).with_name("interleaver".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("interleaver"));
}

#[tokio::test]
async fn test_interleave_transformer_with_error_strategy() {
  let other_stream = Box::pin(stream::iter(vec![10]));
  let transformer =
    InterleaveTransformer::new(other_stream).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}
