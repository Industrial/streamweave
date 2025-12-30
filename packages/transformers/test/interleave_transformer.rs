use futures::{StreamExt, stream};
use streamweave_error::ErrorStrategy;
use streamweave_transformers::InterleaveTransformer;

#[tokio::test]
async fn test_interleave_basic() {
  let other = stream::iter(vec![4, 5, 6].into_iter());
  let mut transformer = InterleaveTransformer::new(Box::pin(other));
  let input = stream::iter(vec![1, 2, 3].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 4, 2, 5, 3, 6]);
}

#[tokio::test]
async fn test_interleave_empty_input() {
  let other = stream::iter(Vec::<i32>::new());
  let mut transformer = InterleaveTransformer::new(Box::pin(other));
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let other = stream::iter(vec![4]);
  let transformer = InterleaveTransformer::new(Box::pin(other))
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}
