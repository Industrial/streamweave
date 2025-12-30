use futures::{StreamExt, stream};
use streamweave_error::ErrorStrategy;
use streamweave_transformers::MapTransformer;

#[tokio::test]
async fn test_map_transformer() {
  let mut transformer = MapTransformer::new(|x: i32| x * 2);
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_map_transformer_type_conversion() {
  let mut transformer = MapTransformer::new(|x: i32| x.to_string());
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<String> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec!["1", "2", "3"]);
}

#[tokio::test]
async fn test_map_transformer_reuse() {
  let mut transformer = MapTransformer::new(|x: i32| x * 2);

  // First transform
  let input1 = stream::iter(vec![1, 2, 3]);
  let boxed_input1 = Box::pin(input1);
  let result1: Vec<i32> = transformer.transform(boxed_input1).await.collect().await;
  assert_eq!(result1, vec![2, 4, 6]);

  // Second transform
  let input2 = stream::iter(vec![4, 5, 6]);
  let boxed_input2 = Box::pin(input2);
  let result2: Vec<i32> = transformer.transform(boxed_input2).await.collect().await;
  assert_eq!(result2, vec![8, 10, 12]);
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let transformer = MapTransformer::new(|x: i32| x * 2)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}
