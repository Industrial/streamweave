use futures::{Stream, StreamExt, stream};
use std::pin::Pin;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_transformers::MergeTransformer;

fn create_stream<T: Send + 'static>(items: Vec<T>) -> Pin<Box<dyn Stream<Item = T> + Send>> {
  Box::pin(stream::iter(items))
}

#[tokio::test]
async fn test_merge_empty_input() {
  let mut transformer = MergeTransformer::<i32>::new();
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let transformer = MergeTransformer::new()
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}

#[test]
fn test_merge_transformer_default() {
  let transformer: MergeTransformer<i32> = MergeTransformer::default();
  assert_eq!(transformer.streams.len(), 0);
  assert_eq!(transformer.config().name(), None);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_merge_transformer_new() {
  let transformer = MergeTransformer::<i32>::new();
  assert_eq!(transformer.streams.len(), 0);
  assert_eq!(transformer.config().name(), None);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_merge_transformer_with_error_strategy() {
  let transformer = MergeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_merge_transformer_with_name() {
  let transformer = MergeTransformer::<i32>::new().with_name("test_merge".to_string());

  assert_eq!(transformer.config().name(), Some("test_merge".to_string()));
}

#[test]
fn test_merge_transformer_add_stream() {
  let mut transformer = MergeTransformer::<i32>::new();
  assert_eq!(transformer.streams.len(), 0);

  let stream1 = create_stream(vec![1, 2, 3]);
  transformer.add_stream(stream1);
  assert_eq!(transformer.streams.len(), 1);

  let stream2 = create_stream(vec![4, 5, 6]);
  transformer.add_stream(stream2);
  assert_eq!(transformer.streams.len(), 2);
}

#[tokio::test]
async fn test_merge_transformer_single_stream() {
  let mut transformer = MergeTransformer::<i32>::new();
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;
  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_merge_transformer_multiple_streams() {
  let mut transformer = MergeTransformer::<i32>::new();

  // Add additional streams
  let stream1 = create_stream(vec![4, 5, 6]);
  let stream2 = create_stream(vec![7, 8, 9]);
  transformer.add_stream(stream1);
  transformer.add_stream(stream2);

  // Main input stream
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  // The result should contain all items from all streams
  // Note: select_all interleaves items from streams, so order may vary
  assert_eq!(result.len(), 9);
  assert!(result.contains(&1));
  assert!(result.contains(&2));
  assert!(result.contains(&3));
  assert!(result.contains(&4));
  assert!(result.contains(&5));
  assert!(result.contains(&6));
  assert!(result.contains(&7));
  assert!(result.contains(&8));
  assert!(result.contains(&9));
}

#[tokio::test]
async fn test_merge_transformer_empty_streams_after_transform() {
  let mut transformer = MergeTransformer::<i32>::new();

  // Add streams
  let stream1 = create_stream(vec![1, 2]);
  let stream2 = create_stream(vec![3, 4]);
  transformer.add_stream(stream1);
  transformer.add_stream(stream2);

  assert_eq!(transformer.streams.len(), 2);

  // Transform should consume the streams
  let input = stream::iter(vec![5, 6]);
  let boxed_input = Box::pin(input);
  let _result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  // Streams should be consumed after transform
  assert_eq!(transformer.streams.len(), 0);
}

#[tokio::test]
async fn test_merge_transformer_reuse() {
  let mut transformer = MergeTransformer::<i32>::new();

  // First use
  let input1 = stream::iter(vec![1, 2]);
  let boxed_input1 = Box::pin(input1);
  let result1: Vec<i32> = transformer.transform(boxed_input1).await.collect().await;
  assert_eq!(result1, vec![1, 2]);

  // Add streams for second use
  let stream1 = create_stream(vec![3, 4]);
  transformer.add_stream(stream1);

  // Second use
  let input2 = stream::iter(vec![5, 6]);
  let boxed_input2 = Box::pin(input2);
  let result2: Vec<i32> = transformer.transform(boxed_input2).await.collect().await;
  assert_eq!(result2.len(), 4);
  assert!(result2.contains(&3));
  assert!(result2.contains(&4));
  assert!(result2.contains(&5));
  assert!(result2.contains(&6));
}
