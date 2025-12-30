use futures::{StreamExt, stream};
use proptest::prelude::*;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_transformers::GroupByTransformer;

#[tokio::test]
async fn test_group_by_basic() {
  let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
  let input = stream::iter(vec![1, 2, 3, 4, 5, 6]);
  let boxed_input = Box::pin(input);

  let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).await.collect().await;
  result.sort_by_key(|(k, _)| *k);

  assert_eq!(result, vec![(0, vec![2, 4, 6]), (1, vec![1, 3, 5])]);
}

#[tokio::test]
async fn test_group_by_empty_input() {
  let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::new());
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let transformer = GroupByTransformer::new(|x: &i32| x % 2)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}

#[test]
fn test_group_by_transformer_new() {
  let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

  assert_eq!(transformer.config().name(), None);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_group_by_transformer_with_error_strategy() {
  let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
    .with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_group_by_transformer_with_name() {
  let transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2)
    .with_name("test_group_by".to_string());

  assert_eq!(
    transformer.config().name(),
    Some("test_group_by".to_string())
  );
}

#[tokio::test]
async fn test_group_by_transformer_single_group() {
  let mut transformer = GroupByTransformer::new(|_x: &i32| 0);
  let input = stream::iter(vec![1, 2, 3, 4, 5]);
  let boxed_input = Box::pin(input);

  let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).await.collect().await;
  assert_eq!(result, vec![(0, vec![1, 2, 3, 4, 5])]);
}

#[tokio::test]
async fn test_group_by_transformer_multiple_groups() {
  let mut transformer = GroupByTransformer::new(|x: &i32| x % 3);
  let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
  let boxed_input = Box::pin(input);

  let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).await.collect().await;
  result.sort_by_key(|(k, _)| *k);

  assert_eq!(
    result,
    vec![(0, vec![3, 6, 9]), (1, vec![1, 4, 7]), (2, vec![2, 5, 8])]
  );
}

#[tokio::test]
async fn test_group_by_transformer_string_keys() {
  let mut transformer = GroupByTransformer::new(|x: &i32| {
    if *x < 5 {
      "small".to_string()
    } else {
      "large".to_string()
    }
  });
  let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  let boxed_input = Box::pin(input);

  let mut result: Vec<(String, Vec<i32>)> =
    transformer.transform(boxed_input).await.collect().await;
  result.sort_by_key(|(k, _)| k.clone());

  assert_eq!(
    result,
    vec![
      ("large".to_string(), vec![5, 6, 7, 8, 9, 10]),
      ("small".to_string(), vec![1, 2, 3, 4])
    ]
  );
}

#[tokio::test]
async fn test_group_by_transformer_duplicate_keys() {
  let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
  let input = stream::iter(vec![1, 1, 1, 2, 2, 2]);
  let boxed_input = Box::pin(input);

  let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).await.collect().await;
  result.sort_by_key(|(k, _)| *k);

  assert_eq!(result, vec![(0, vec![2, 2, 2]), (1, vec![1, 1, 1])]);
}

#[tokio::test]
async fn test_group_by_transformer_reuse() {
  let mut transformer = GroupByTransformer::<fn(&i32) -> i32, i32, i32>::new(|x| x % 2);

  // First use
  let input1 = stream::iter(vec![1, 2, 3]);
  let boxed_input1 = Box::pin(input1);
  let mut result1: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input1).await.collect().await;
  result1.sort_by_key(|(k, _)| *k);
  assert_eq!(result1, vec![(0, vec![2]), (1, vec![1, 3])]);

  // Second use
  let input2 = stream::iter(vec![4, 5, 6]);
  let boxed_input2 = Box::pin(input2);
  let mut result2: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input2).await.collect().await;
  result2.sort_by_key(|(k, _)| *k);
  assert_eq!(result2, vec![(0, vec![4, 6]), (1, vec![5])]);
}
