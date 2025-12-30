use futures::{StreamExt, stream};
use streamweave_error::ErrorStrategy;
use streamweave_transformers::ReduceTransformer;

#[tokio::test]
async fn test_reduce_transformer_sum() {
  let mut transformer = ReduceTransformer::new(0, |acc, x| acc + x);
  let input = vec![1, 2, 3, 4, 5];
  let input_stream = Box::pin(stream::iter(input.into_iter()));

  let result: Vec<i32> = transformer.transform(input_stream).await.collect().await;

  assert_eq!(result, vec![1, 3, 6, 10, 15]);
}

#[tokio::test]
async fn test_reduce_transformer_string_concat() {
  let mut transformer = ReduceTransformer::new(String::new(), |acc, x| acc + x);
  let input = vec!["a", "b", "c"];
  let input_stream = Box::pin(stream::iter(input.into_iter()));

  let result: Vec<String> = transformer.transform(input_stream).await.collect().await;

  assert_eq!(
    result,
    vec!["a".to_string(), "ab".to_string(), "abc".to_string()]
  );
}

#[tokio::test]
async fn test_reduce_transformer_with_error() {
  let mut transformer = ReduceTransformer::new(0, |acc, x| if x % 2 == 0 { acc + x } else { acc });

  let input = vec![2, 3, 4, 5, 6];
  let input_stream = Box::pin(stream::iter(input.into_iter()));

  let result = transformer
    .transform(input_stream)
    .await
    .collect::<Vec<_>>()
    .await;

  assert_eq!(result, vec![2, 2, 6, 6, 12]);
}

#[tokio::test]
async fn test_reduce_transformer_custom_type() {
  #[derive(Debug, Clone, PartialEq)]
  struct Counter {
    count: i32,
    sum: i32,
  }

  let mut transformer = ReduceTransformer::new(Counter { count: 0, sum: 0 }, |acc, x| Counter {
    count: acc.count + 1,
    sum: acc.sum + x,
  });

  let input = vec![1, 2, 3, 4, 5];
  let input_stream = Box::pin(stream::iter(input.into_iter()));

  let result: Vec<Counter> = transformer.transform(input_stream).await.collect().await;

  assert_eq!(
    result,
    vec![
      Counter { count: 1, sum: 1 },
      Counter { count: 2, sum: 3 },
      Counter { count: 3, sum: 6 },
      Counter { count: 4, sum: 10 },
      Counter { count: 5, sum: 15 },
    ]
  );
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let transformer = ReduceTransformer::new(0, |acc, x| acc + x)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}
