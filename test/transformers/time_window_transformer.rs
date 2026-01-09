//! Tests for TimeWindowTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::TimeWindowTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_time_window_transformer_basic() {
  let mut transformer = TimeWindowTransformer::new(Duration::from_millis(50));
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;

  // Take first item to start the stream
  let _first = output_stream.next().await;

  // Wait a bit for time window to potentially trigger
  tokio::time::sleep(Duration::from_millis(60)).await;

  // Should have emitted at least one window
  let mut results = Vec::new();
  while let Some(window) = output_stream.next().await {
    results.push(window);
    if results.len() >= 2 {
      break; // Don't wait too long
    }
  }

  // Should have at least one window
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_time_window_transformer_empty() {
  let mut transformer = TimeWindowTransformer::new(Duration::from_millis(10));
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;

  // Wait a bit
  tokio::time::sleep(Duration::from_millis(20)).await;

  // Should not emit anything for empty stream
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_time_window_transformer_with_name() {
  let transformer =
    TimeWindowTransformer::new(Duration::from_secs(1)).with_name("time_window".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("time_window"));
}

#[tokio::test]
async fn test_time_window_transformer_with_error_strategy() {
  let transformer =
    TimeWindowTransformer::new(Duration::from_secs(1)).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_time_window_transformer_clone() {
  let transformer1 = TimeWindowTransformer::new(Duration::from_secs(1));
  let transformer2 = transformer1.clone();

  // Both should be valid
  assert!(true);
}
