//! Tests for TimeWindowTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave_error::ErrorStrategy;
use streamweave_window::TimeWindowTransformer;

#[tokio::test]
async fn test_time_window_transformer_basic() {
  let mut transformer = TimeWindowTransformer::new(Duration::from_millis(100));
  let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  // Should have at least one window
  assert!(!result.is_empty());
  // Should have received all items
  let total_items: usize = result.iter().map(|w| w.len()).sum();
  assert_eq!(total_items, 5);
}

#[tokio::test]
async fn test_time_window_transformer_empty_input() {
  let mut transformer = TimeWindowTransformer::new(Duration::from_millis(100));
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<Vec<i32>>::new());
}

#[tokio::test]
async fn test_time_window_transformer_with_error_strategy() {
  let transformer = TimeWindowTransformer::new(Duration::from_secs(1))
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}
