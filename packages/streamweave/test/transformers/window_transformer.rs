//! Tests for WindowTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::WindowTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_window_transformer_basic() {
  let mut transformer = WindowTransformer::new(3);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6, 7]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(window) = output_stream.next().await {
    results.push(window);
  }

  // Should have windows: [1,2,3], [2,3,4], [3,4,5], [4,5,6], [5,6,7], [6,7], [7]
  assert!(!results.is_empty());
  assert_eq!(results[0], vec![1, 2, 3]);
  if results.len() > 1 {
    assert_eq!(results[1], vec![2, 3, 4]);
  }
}

#[tokio::test]
async fn test_window_transformer_size_one() {
  let mut transformer = WindowTransformer::new(1);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(window) = output_stream.next().await {
    results.push(window);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], vec![1]);
  assert_eq!(results[1], vec![2]);
  assert_eq!(results[2], vec![3]);
}

#[tokio::test]
async fn test_window_transformer_less_than_window_size() {
  let mut transformer = WindowTransformer::new(5);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(window) = output_stream.next().await {
    results.push(window);
  }

  // Should emit partial window
  assert_eq!(results.len(), 1);
  assert_eq!(results[0], vec![1, 2, 3]);
}

#[tokio::test]
async fn test_window_transformer_empty() {
  let mut transformer = WindowTransformer::new(3);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_window_transformer_with_name() {
  let transformer = WindowTransformer::new(3).with_name("window_3".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("window_3"));
}

#[tokio::test]
async fn test_window_transformer_with_error_strategy() {
  let transformer = WindowTransformer::new(3).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}
