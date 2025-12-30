use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::Transformer;
use streamweave_transformers::ZipTransformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_zip_basic() {
    let mut transformer = ZipTransformer::new();
    let input = stream::iter(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![vec![1, 4, 7], vec![2, 5, 8], vec![3, 6, 9]]);
  }

  #[tokio::test]
  async fn test_zip_empty_input() {
    let mut transformer = ZipTransformer::new();
    let input = stream::iter(Vec::<Vec<i32>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }
}
