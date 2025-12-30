use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::SplitAtTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_split_at_basic() {
    let mut transformer = SplitAtTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![(vec![1, 2], vec![3, 4, 5])]);
  }

  #[tokio::test]
  async fn test_split_at_empty_input() {
    let mut transformer = SplitAtTransformer::new(2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(Vec<i32>, Vec<i32>)> =
      transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::new());
  }
}
