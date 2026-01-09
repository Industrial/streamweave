//! Tests for CircuitBreakerTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::CircuitBreakerTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_circuit_breaker_transformer_basic() {
  let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(50));
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_circuit_breaker_transformer_empty() {
  let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(50));
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_circuit_breaker_transformer_with_name() {
  let transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(50))
    .with_name("circuit_breaker".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("circuit_breaker"));
}

#[tokio::test]
async fn test_circuit_breaker_transformer_with_error_strategy() {
  let transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(50))
    .with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_circuit_breaker_transformer_clone() {
  let transformer1 = CircuitBreakerTransformer::new(3, Duration::from_millis(50));
  let transformer2 = transformer1.clone();

  assert_eq!(transformer2.failure_threshold, 3);
  assert_eq!(transformer2.reset_timeout, Duration::from_millis(50));
}
