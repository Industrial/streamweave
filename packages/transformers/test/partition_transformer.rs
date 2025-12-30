use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::PartitionTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;
  use streamweave_error::ErrorStrategy;

  #[tokio::test]
  async fn test_partition_basic() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![2, 4, 6], vec![1, 3, 5])]);
  }

  #[tokio::test]
  async fn test_partition_empty_input() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, Some("test_transformer".to_string()));
  }

  // Test partition with all even numbers
  #[tokio::test]
  async fn test_partition_all_even() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![2, 4, 6, 8, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![2, 4, 6, 8, 10], vec![])]);
  }

  // Test partition with all odd numbers
  #[tokio::test]
  async fn test_partition_all_odd() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 3, 5, 7, 9]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![], vec![1, 3, 5, 7, 9])]);
  }

  // Test partition with single element
  #[tokio::test]
  async fn test_partition_single_element() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![42], vec![])]);
  }

  // Test partition with single odd element
  #[tokio::test]
  async fn test_partition_single_odd_element() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![43]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![], vec![43])]);
  }

  // Test partition with strings
  #[tokio::test]
  async fn test_partition_with_strings() {
    let mut transformer = PartitionTransformer::new(|x: &String| x.len() > 3);
    let input = stream::iter(vec![
      "a".to_string(),
      "bb".to_string(),
      "ccc".to_string(),
      "dddd".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<String>, Vec<String>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(
      result,
      vec![(
        vec!["dddd".to_string()],
        vec!["a".to_string(), "bb".to_string(), "ccc".to_string()]
      )]
    );
  }

  // Test partition with floats
  #[tokio::test]
  async fn test_partition_with_floats() {
    let mut transformer = PartitionTransformer::new(|x: &f64| *x > 0.0);
    let input = stream::iter(vec![-1.5, 0.0, 2.7, -3.2, 4.1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<f64>, Vec<f64>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![2.7, 4.1], vec![-1.5, 0.0, -3.2])]);
  }

  // Test partition with custom predicate
  #[tokio::test]
  async fn test_partition_custom_predicate() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x > 5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![6, 7, 8, 9, 10], vec![1, 2, 3, 4, 5])]);
  }

  // Test partition with zero values
  #[tokio::test]
  async fn test_partition_with_zero_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x == 0);
    let input = stream::iter(vec![0, 1, 0, 2, 0, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![0, 0, 0], vec![1, 2, 3])]);
  }

  // Test partition with negative numbers
  #[tokio::test]
  async fn test_partition_with_negative_numbers() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x < 0);
    let input = stream::iter(vec![-1, 2, -3, 4, -5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![-1, -3, -5], vec![2, 4])]);
  }

  // Test partition with large numbers
  #[tokio::test]
  async fn test_partition_with_large_numbers() {
    let mut transformer = PartitionTransformer::new(|x: &i64| *x > i64::MAX / 2);
    let input = stream::iter(vec![i64::MAX, i64::MIN, 0, 1]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i64>, Vec<i64>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![i64::MAX], vec![i64::MIN, 0, 1])]);
  }

  // Test partition with duplicate values
  #[tokio::test]
  async fn test_partition_with_duplicate_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x % 2 == 0);
    let input = stream::iter(vec![1, 1, 2, 2, 3, 3, 4, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![2, 2, 4, 4], vec![1, 1, 3, 3])]);
  }

  // Test partition with ascending values
  #[tokio::test]
  async fn test_partition_with_ascending_values() {
    let mut transformer = PartitionTransformer::new(|x: &i32| *x < 5);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![1, 2, 3, 4], vec![5, 6, 7, 8, 9, 10])]);
  }
}
