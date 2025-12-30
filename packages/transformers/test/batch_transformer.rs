use futures::stream;
use streamweave_error::ErrorStrategy;
use streamweave_transformers::BatchTransformer;

#[tokio::test]
async fn test_batch_exact_size() {
  let mut transformer = BatchTransformer::new(3).unwrap();
  let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6]]);
}

#[tokio::test]
async fn test_batch_partial_last_chunk() {
  let mut transformer = BatchTransformer::new(2).unwrap();
  let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
}

#[tokio::test]
async fn test_batch_empty_input() {
  let mut transformer = BatchTransformer::new(2).unwrap();
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert!(result.is_empty());
}

#[tokio::test]
async fn test_batch_size_one() {
  let mut transformer = BatchTransformer::new(1).unwrap();
  let input = stream::iter(vec![1, 2, 3].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
}

#[tokio::test]
async fn test_batch_size_larger_than_input() {
  let mut transformer = BatchTransformer::new(10).unwrap();
  let input = stream::iter(vec![1, 2, 3].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![vec![1, 2, 3]]);
}

#[test]
fn test_batch_invalid_size() {
  let result = BatchTransformer::<i32>::new(0);
  assert!(result.is_err());
}

#[test]
fn test_error_handling_strategies() {
  let transformer: BatchTransformer<i32> = BatchTransformer::new(2).unwrap();
  assert_eq!(transformer.config.error_strategy, ErrorStrategy::Stop);
}
