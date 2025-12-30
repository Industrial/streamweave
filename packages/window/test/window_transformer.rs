//! Tests for WindowTransformer

use futures::StreamExt;
use futures::stream;
use streamweave_error::ErrorStrategy;
use streamweave_window::WindowTransformer;

#[tokio::test]
async fn test_window_transformer_basic() {
  let mut transformer = WindowTransformer::new(3);
  let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  // Sliding window of size 3: each window overlaps by 2 items
  // The transformer also emits a partial window at the end if there are remaining items
  assert_eq!(
    result,
    vec![
      vec![1, 2, 3],
      vec![2, 3, 4],
      vec![3, 4, 5],
      vec![4, 5, 6],
      vec![5, 6, 7],
      vec![6, 7] // Partial window at the end
    ]
  );
}

#[tokio::test]
async fn test_window_transformer_empty_input() {
  let mut transformer = WindowTransformer::new(3);
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<Vec<i32>>::new());
}

#[tokio::test]
async fn test_window_transformer_with_error_strategy() {
  let transformer = WindowTransformer::new(3)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}

#[tokio::test]
async fn test_window_transformer_size_one() {
  let mut transformer = WindowTransformer::new(1);
  let input = stream::iter(vec![1, 2, 3].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
}

#[tokio::test]
async fn test_window_transformer_larger_than_input() {
  let mut transformer = WindowTransformer::new(10);
  let input = stream::iter(vec![1, 2, 3].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  // Should emit partial window at the end
  assert_eq!(result, vec![vec![1, 2, 3]]);
}
