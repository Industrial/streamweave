use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::SkipTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_skip_basic() {
    let mut transformer = SkipTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![3, 4, 5]);
  }

  #[tokio::test]
  async fn test_skip_empty_input() {
    let mut transformer = SkipTransformer::new(2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }
}
