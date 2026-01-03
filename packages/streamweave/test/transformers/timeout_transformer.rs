//! Tests for TimeoutTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::TimeoutTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_timeout_transformer_basic() {
  let mut transformer = TimeoutTransformer::new(Duration::from_secs(1));
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_timeout_transformer_empty() {
  let mut transformer = TimeoutTransformer::new(Duration::from_secs(1));
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_timeout_transformer_with_name() {
  let transformer =
    TimeoutTransformer::new(Duration::from_secs(1)).with_name("timeout".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("timeout"));
}

#[tokio::test]
async fn test_timeout_transformer_with_error_strategy() {
  let transformer =
    TimeoutTransformer::new(Duration::from_secs(1)).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_timeout_transformer_clone() {
  let transformer1 = TimeoutTransformer::new(Duration::from_secs(1));
  let transformer2 = transformer1.clone();

  assert_eq!(transformer2.duration, Duration::from_secs(1));
}
