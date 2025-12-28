use super::ordered_merge_transformer::{MergeStrategy, OrderedMergeTransformer};
use async_stream::stream;
use async_trait::async_trait;
use futures::{StreamExt, stream::select_all};
use streamweave::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T> Transformer for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let strategy = self.strategy.clone();
    let mut all_streams = vec![input];
    all_streams.extend(std::mem::take(&mut self.streams));

    match strategy {
      MergeStrategy::Interleave => {
        // Use select_all for fair interleaving
        Box::pin(select_all(all_streams))
      }

      MergeStrategy::Sequential => {
        // Process streams in order, exhaust each before moving to next
        Box::pin(stream! {
            for mut stream in all_streams {
                while let Some(item) = stream.next().await {
                    yield item;
                }
            }
        })
      }

      MergeStrategy::RoundRobin => {
        // Take one element from each stream in turn
        Box::pin(stream! {
            let mut streams: Vec<_> = all_streams.into_iter().map(Some).collect();
            let mut active_count = streams.len();

            loop {
                if active_count == 0 {
                    break;
                }

                for stream_option in &mut streams {
                    if let Some(stream) = stream_option {
                        match stream.next().await {
                            Some(item) => yield item,
                            None => {
                                *stream_option = None;
                                active_count -= 1;
                            }
                        }
                    }
                }
            }
        })
      }

      MergeStrategy::Priority => {
        // Process streams based on priority (lower index = higher priority)
        // When higher priority stream has elements, process them first
        Box::pin(stream! {
            let mut streams: Vec<_> = all_streams.into_iter().map(Some).collect();
            let mut active_count = streams.len();

            loop {
                if active_count == 0 {
                    break;
                }

                // Try to get from highest priority stream first
                let mut got_item = false;
                for stream_option in &mut streams {
                    if let Some(stream) = stream_option {
                        match stream.next().await {
                            Some(item) => {
                                yield item;
                                got_item = true;
                                break; // After yielding from highest priority, check again
                            }
                            None => {
                                *stream_option = None;
                                active_count -= 1;
                            }
                        }
                    }
                }

                // If we didn't get any item and still have active streams,
                // they're all waiting for async operations - this shouldn't happen
                // with synchronous stream::iter, but let's be safe
                if !got_item && active_count > 0 {
                    // All active streams are blocked, yield control
                    tokio::task::yield_now().await;
                }
            }
        })
      }
    }
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "ordered_merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Priority: highest priority stream (input) is exhausted first
    // For sync streams, this behaves like sequential
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
  }

  #[tokio::test]
  async fn test_merge_round_robin_uneven_streams() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

    transformer.add_stream(create_stream(vec![4, 5])); // Only 2 elements
    transformer.add_stream(create_stream(vec![7, 8, 9, 10])); // 4 elements

    let input = stream::iter(vec![1, 2, 3]); // 3 elements
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Round-robin with uneven streams
    // First pass: 1, 4, 7
    // Second pass: 2, 5, 8
    // Third pass: 3, (stream 1 done), 9
    // Fourth pass: (stream 0 done), (stream 1 done), 10
    assert_eq!(result.len(), 9);
    assert!(result.contains(&1));
    assert!(result.contains(&10));
  }

  #[tokio::test]
  async fn test_merge_sequential_empty_streams() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Sequential);

    transformer.add_stream(create_stream(Vec::<i32>::new())); // Empty
    transformer.add_stream(create_stream(vec![4, 5]));

    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Empty stream is skipped
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_merge_empty_input() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Interleave);

    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_merge_single_stream() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_merge_with_strings() {
    let mut transformer =
      OrderedMergeTransformer::<String>::new().with_strategy(MergeStrategy::Sequential);

    transformer.add_stream(create_stream(vec!["d".to_string(), "e".to_string()]));

    let input = stream::iter(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
        "e".to_string()
      ]
    );
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
    let transformer =
      OrderedMergeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Stop);

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
    let transformer =
      OrderedMergeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

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

  #[tokio::test]
  async fn test_merge_reusability() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::Sequential);

    // First use
    let input1 = stream::iter(vec![1, 2]);
    let result1: Vec<i32> = transformer.transform(Box::pin(input1)).collect().await;
    assert_eq!(result1, vec![1, 2]);

    // Add new stream for second use
    transformer.add_stream(create_stream(vec![3, 4]));

    let input2 = stream::iter(vec![5, 6]);
    let result2: Vec<i32> = transformer.transform(Box::pin(input2)).collect().await;
    assert_eq!(result2, vec![5, 6, 3, 4]);
  }

  #[tokio::test]
  async fn test_merge_all_streams_complete() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

    transformer.add_stream(create_stream(vec![3]));

    let input = stream::iter(vec![1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Both streams complete, all elements should be processed
    assert_eq!(result.len(), 3);
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
  }

  #[tokio::test]
  async fn test_merge_preserves_order_within_stream() {
    let mut transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);

    transformer.add_stream(create_stream(vec![4, 5, 6]));

    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Check that elements from each stream maintain their relative order
    let stream0_elements: Vec<i32> = result.iter().cloned().filter(|&x| x <= 3).collect();
    let stream1_elements: Vec<i32> = result.iter().cloned().filter(|&x| x > 3).collect();

    assert_eq!(stream0_elements, vec![1, 2, 3]);
    assert_eq!(stream1_elements, vec![4, 5, 6]);
  }
}
