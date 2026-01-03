//! Tests for RetryTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::RetryTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_retry_transformer_basic() {
  let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_retry_transformer_empty() {
  let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_retry_transformer_max_retries() {
  let transformer = RetryTransformer::new(5, Duration::from_millis(10));
  assert_eq!(transformer.max_retries(), 5);
}

#[tokio::test]
async fn test_retry_transformer_backoff() {
  let transformer = RetryTransformer::new(3, Duration::from_secs(1));
  assert_eq!(transformer.backoff(), Duration::from_secs(1));
}

#[tokio::test]
async fn test_retry_transformer_with_name() {
  let transformer =
    RetryTransformer::new(3, Duration::from_millis(10)).with_name("retry".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("retry"));
}

#[tokio::test]
async fn test_retry_transformer_with_error_strategy() {
  let transformer =
    RetryTransformer::new(3, Duration::from_millis(10)).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_retry_transformer_clone() {
  let transformer1 = RetryTransformer::new(3, Duration::from_millis(10));
  let transformer2 = transformer1.clone();

  assert_eq!(transformer2.max_retries(), 3);
  assert_eq!(transformer2.backoff(), Duration::from_millis(10));
}
