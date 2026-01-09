//! Tests for RateLimitTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::RateLimitTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_rate_limit_transformer_basic() {
  let mut transformer = RateLimitTransformer::new(10, Duration::from_secs(1));
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should allow items within rate limit
  assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_rate_limit_transformer_empty() {
  let mut transformer = RateLimitTransformer::new(10, Duration::from_secs(1));
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_rate_limit_transformer_with_name() {
  let transformer =
    RateLimitTransformer::new(10, Duration::from_secs(1)).with_name("rate_limiter".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("rate_limiter"));
}

#[tokio::test]
async fn test_rate_limit_transformer_with_error_strategy() {
  let transformer =
    RateLimitTransformer::new(10, Duration::from_secs(1)).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}
