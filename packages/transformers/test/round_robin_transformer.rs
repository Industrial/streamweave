use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::RoundRobinTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;
  use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

  #[tokio::test]
  async fn test_round_robin_basic() {
    let mut transformer = RoundRobinTransformer::<i32>::new(3);
    let input = stream::iter(vec![10, 20, 30, 40, 50, 60]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result.len(), 6);
    assert_eq!(result[0], (0, 10));
    assert_eq!(result[1], (1, 20));
    assert_eq!(result[2], (2, 30));
    assert_eq!(result[3], (0, 40));
    assert_eq!(result[4], (1, 50));
    assert_eq!(result[5], (2, 60));
  }

  #[tokio::test]
  async fn test_round_robin_two_consumers() {
    let mut transformer = RoundRobinTransformer::<i32>::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result.len(), 5);
    assert_eq!(result[0], (0, 1));
    assert_eq!(result[1], (1, 2));
    assert_eq!(result[2], (0, 3));
    assert_eq!(result[3], (1, 4));
    assert_eq!(result[4], (0, 5));
  }

  #[tokio::test]
  async fn test_round_robin_empty_input() {
    let mut transformer = RoundRobinTransformer::<i32>::new(3);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(usize, i32)> = transformer.transform(boxed_input).await.collect().await;

    assert!(result.is_empty());
  }
}
