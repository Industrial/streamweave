use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave_transformers::Input;
use streamweave_transformers::TimeoutTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_timeout_basic() {
    let mut transformer = TimeoutTransformer::<i32>::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_timeout_empty_input() {
    let mut transformer = TimeoutTransformer::<i32>::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }
}
