use futures::{StreamExt, stream};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_transformers::{MergeStrategy, OrderedMergeTransformer};

fn create_stream<T: Send + 'static>(
  items: Vec<T>,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>> {
  Box::pin(stream::iter(items))
}

#[tokio::test]
async fn test_merge_interleave() {
  let mut transformer =
    OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Interleave);

  transformer.add_stream(create_stream(vec![4, 5, 6]));
  transformer.add_stream(create_stream(vec![7, 8, 9]));

  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  // Interleave doesn't guarantee order, but all elements should be present
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
async fn test_merge_sequential() {
  let mut transformer =
    OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Sequential);

  transformer.add_stream(create_stream(vec![4, 5, 6]));
  transformer.add_stream(create_stream(vec![7, 8, 9]));

  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  // Sequential: first stream exhausted, then second, then third
  assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[tokio::test]
async fn test_merge_round_robin() {
  let mut transformer =
    OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

  transformer.add_stream(create_stream(vec![4, 5, 6]));
  transformer.add_stream(create_stream(vec![7, 8, 9]));

  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  // Round-robin: 1 (stream 0), 4 (stream 1), 7 (stream 2), 2 (stream 0), ...
  assert_eq!(result, vec![1, 4, 7, 2, 5, 8, 3, 6, 9]);
}

#[tokio::test]
async fn test_merge_priority() {
  let mut transformer =
    OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Priority);

  transformer.add_stream(create_stream(vec![4, 5, 6]));
  transformer.add_stream(create_stream(vec![7, 8, 9]));

  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  // Priority: highest priority stream (input) is exhausted first
  // For sync streams, this behaves like sequential
  assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[tokio::test]
async fn test_merge_empty_input() {
  let mut transformer =
    OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Interleave);

  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert!(result.is_empty());
}

#[tokio::test]
async fn test_merge_single_stream() {
  let mut transformer =
    OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_merge_component_info() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_name("my_merger".to_string());

  let info = transformer.component_info();
  assert_eq!(info.name, "my_merger");
  assert!(info.type_name.contains("OrderedMergeTransformer"));
}

#[test]
fn test_merge_default_component_info() {
  let transformer = OrderedMergeTransformer::<i32>::new();

  let info = transformer.component_info();
  assert_eq!(info.name, "ordered_merge_transformer");
}

#[test]
fn test_merge_error_handling_stop() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Stop);

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "OrderedMergeTransformer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "OrderedMergeTransformer".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Stop
  ));
}

#[test]
fn test_merge_error_handling_skip() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "OrderedMergeTransformer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "OrderedMergeTransformer".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Skip
  ));
}

#[test]
fn test_merge_create_error_context() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_name("test_merger".to_string());

  let context = transformer.create_error_context(Some(42));
  assert_eq!(context.component_name, "test_merger");
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_merge_strategy_default() {
  let strategy = MergeStrategy::default();
  assert_eq!(strategy, MergeStrategy::Interleave);
}

#[test]
fn test_ordered_merge_transformer_new() {
  let transformer = OrderedMergeTransformer::<i32>::new();
  assert_eq!(&transformer.strategy(), &MergeStrategy::Interleave);
  assert_eq!(transformer.stream_count(), 0);
}

#[test]
fn test_ordered_merge_transformer_with_strategy() {
  let transformer = OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);
  assert_eq!(&transformer.strategy(), &MergeStrategy::RoundRobin);
}

#[test]
fn test_ordered_merge_transformer_add_stream() {
  let mut transformer = OrderedMergeTransformer::<i32>::new();
  let initial_count = transformer.stream_count();
  let stream = Box::pin(stream::iter(vec![1, 2, 3]));
  transformer.add_stream(stream);
  assert_eq!(transformer.stream_count(), initial_count + 1);
}

#[test]
fn test_ordered_merge_transformer_default() {
  let transformer = OrderedMergeTransformer::<i32>::default();
  assert_eq!(&transformer.strategy(), &MergeStrategy::Interleave);
  assert_eq!(transformer.stream_count(), 0);
}
