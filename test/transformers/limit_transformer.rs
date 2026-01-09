//! Tests for LimitTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::LimitTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_limit_transformer_basic() {
  let mut transformer = LimitTransformer::new(3);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_limit_transformer_more_than_available() {
  let mut transformer = LimitTransformer::new(10);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_limit_transformer_zero() {
  let mut transformer = LimitTransformer::new(0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_limit_transformer_empty() {
  let mut transformer = LimitTransformer::new(3);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_limit_transformer_with_name() {
  let transformer = LimitTransformer::new(5).with_name("limit_5".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("limit_5"));
}

#[tokio::test]
async fn test_limit_transformer_with_error_strategy() {
  let transformer = LimitTransformer::new(5).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}
