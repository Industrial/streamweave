use futures::{StreamExt, stream};
use streamweave_error::ErrorStrategy;
use streamweave_transformers::FilterTransformer;

#[tokio::test]
async fn test_filter_even_numbers() {
  let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
  let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_filter_empty_input() {
  let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_filter_all_match() {
  let mut transformer = FilterTransformer::new(|x: &i32| *x > 0);
  let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_filter_none_match() {
  let mut transformer = FilterTransformer::new(|x: &i32| *x > 10);
  let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_filter_with_strings() {
  let mut transformer = FilterTransformer::new(|s: &String| s.starts_with("a"));
  let input = stream::iter(
    vec![
      "apple".to_string(),
      "banana".to_string(),
      "avocado".to_string(),
      "cherry".to_string(),
    ]
    .into_iter(),
  );
  let boxed_input = Box::pin(input);

  let result: Vec<String> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec!["apple".to_string(), "avocado".to_string()]);
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let transformer = FilterTransformer::new(|_: &i32| true)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}
