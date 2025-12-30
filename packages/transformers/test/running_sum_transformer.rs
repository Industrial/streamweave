use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::RunningSumTransformer;
use streamweave_transformers::Transformer;

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_running_sum_basic() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

    let output = transformer.transform(input).await;
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_running_sum_with_initial_value() {
    let mut transformer = RunningSumTransformer::<i32>::with_initial(100);
    let input = Box::pin(stream::iter(vec![1, 2, 3]));

    let output = transformer.transform(input).await;
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![101, 103, 106]);
  }

  #[tokio::test]
  async fn test_running_sum_empty_input() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input: std::pin::Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
      Box::pin(stream::iter(vec![]));

    let output = transformer.transform(input).await;
    let results: Vec<i32> = output.collect().await;

    assert!(results.is_empty());
  }

  #[tokio::test]
  async fn test_running_sum_floats() {
    let mut transformer = RunningSumTransformer::<f64>::new();
    let input = Box::pin(stream::iter(vec![1.5, 2.5, 3.0]));

    let output = transformer.transform(input).await;
    let results: Vec<f64> = output.collect().await;

    assert_eq!(results, vec![1.5, 4.0, 7.0]);
  }

  #[tokio::test]
  async fn test_running_sum_state_persistence() {
    let mut transformer = RunningSumTransformer::<i32>::new();

    // First batch
    let input1 = Box::pin(stream::iter(vec![1, 2, 3]));
    let output1 = transformer.transform(input1).await;
    let results1: Vec<i32> = output1.collect().await;
    assert_eq!(results1, vec![1, 3, 6]);

    // State should persist, so second batch continues from 6
    let input2 = Box::pin(stream::iter(vec![4, 5]));
    let output2 = transformer.transform(input2).await;
    let results2: Vec<i32> = output2.collect().await;
    assert_eq!(results2, vec![10, 15]);
  }

  #[tokio::test]
  async fn test_running_sum_state_reset() {
    let mut transformer = RunningSumTransformer::<i32>::new();

    // Process some items
    let input1 = Box::pin(stream::iter(vec![1, 2, 3]));
    let output1 = transformer.transform(input1).await;
    let _: Vec<i32> = output1.collect().await;

    // Reset state
    transformer.reset_state().unwrap();

    // Should start from 0 again
    let input2 = Box::pin(stream::iter(vec![10, 20]));
    let output2 = transformer.transform(input2).await;
    let results2: Vec<i32> = output2.collect().await;
    assert_eq!(results2, vec![10, 30]);
  }

  #[tokio::test]
  async fn test_running_sum_get_state() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

    let output = transformer.transform(input).await;
    let _: Vec<i32> = output.collect().await;

    // Get final state
    let final_state = transformer.state().unwrap().unwrap();
    assert_eq!(final_state, 15);
  }

  #[tokio::test]
  async fn test_running_sum_negative_numbers() {
    let mut transformer = RunningSumTransformer::<i32>::new();
    let input = Box::pin(stream::iter(vec![10, -3, 5, -7]));

    let output = transformer.transform(input).await;
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![10, 7, 12, 5]);
  }

  #[tokio::test]
  async fn test_running_sum_set_state() {
    let mut transformer = RunningSumTransformer::<i32>::new();

    // Set state directly
    transformer.set_state(100).unwrap();

    // Process some items
    let input = Box::pin(stream::iter(vec![1, 2, 3]));
    let output = transformer.transform(input).await;
    let results: Vec<i32> = output.collect().await;

    assert_eq!(results, vec![101, 103, 106]);
  }
}
