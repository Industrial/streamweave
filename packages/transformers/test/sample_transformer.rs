use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::SampleTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_sample_probability_one() {
    let mut transformer = SampleTransformer::new(1.0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    // With probability 1.0, all items should be emitted
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_sample_empty_input() {
    let mut transformer = SampleTransformer::new(0.5);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result.len(), 0);
  }
}
