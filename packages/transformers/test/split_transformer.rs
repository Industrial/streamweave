use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::SplitTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_split_by_even_numbers() {
    let mut transformer = SplitTransformer::new(|x: &i32| x % 2 == 0);
    let input = vec![vec![1, 2, 3, 4, 5, 6]];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<Vec<i32>> = transformer.transform(input_stream).await.collect().await;

    assert_eq!(result, vec![vec![1], vec![2, 3], vec![4, 5], vec![6]]);
  }

  #[tokio::test]
  async fn test_split_empty_input() {
    let mut transformer = SplitTransformer::new(|x: &i32| x % 2 == 0);
    let input = vec![vec![]];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<Vec<i32>> = transformer.transform(input_stream).await.collect().await;

    assert_eq!(result, vec![vec![]]);
  }
}
