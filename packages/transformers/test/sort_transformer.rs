use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::SortTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_sort_basic() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(vec![3, 1, 4, 1, 5, 9]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![1, 1, 3, 4, 5, 9]);
  }

  #[tokio::test]
  async fn test_sort_empty_input() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }
}
