use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::merge::merge_transformer::MergeTransformer;
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;

#[async_trait]
impl<T> Transformer for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut all_streams = vec![input];
    all_streams.extend(std::mem::take(&mut self.streams));
    Box::pin(futures::stream::select_all(all_streams))
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
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::Stream;
  use futures::StreamExt;
  use futures::stream;
  use proptest::prelude::*;
  use std::pin::Pin;

  fn create_stream<T: Send + 'static>(items: Vec<T>) -> Pin<Box<dyn Stream<Item = T> + Send>> {
    Box::pin(stream::iter(items))
  }

  #[tokio::test]
  async fn test_merge_empty_input() {
    let mut transformer = MergeTransformer::<i32>::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_merge_empty_streams() {
    let mut transformer = MergeTransformer::<i32>::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
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

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

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
  async fn test_merge_transformer_mixed_types() {
    let mut transformer = MergeTransformer::<String>::new();

    let stream1 = create_stream(vec!["a".to_string(), "b".to_string()]);
    let stream2 = create_stream(vec!["c".to_string(), "d".to_string()]);
    transformer.add_stream(stream1);
    transformer.add_stream(stream2);

    let input = stream::iter(vec!["x".to_string(), "y".to_string()]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result.len(), 6);
    assert!(result.contains(&"a".to_string()));
    assert!(result.contains(&"b".to_string()));
    assert!(result.contains(&"c".to_string()));
    assert!(result.contains(&"d".to_string()));
    assert!(result.contains(&"x".to_string()));
    assert!(result.contains(&"y".to_string()));
  }

  #[test]
  fn test_merge_transformer_error_handling() {
    let transformer = MergeTransformer::<i32>::new();

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "MergeTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "MergeTransformer".to_string(),
      },
      retries: 0,
    };

    // Test default error strategy (Stop)
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));

    // Test Skip strategy
    let transformer = transformer.with_error_strategy(ErrorStrategy::Skip);
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Skip
    ));

    // Test Retry strategy
    let transformer = transformer.with_error_strategy(ErrorStrategy::Retry(3));
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Retry
    ));

    // Test Retry strategy exhausted
    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "MergeTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "MergeTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_merge_transformer_error_context_creation() {
    let transformer = MergeTransformer::<i32>::new().with_name("test_merge".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_merge");
    assert_eq!(
      context.component_type,
      std::any::type_name::<MergeTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_merge_transformer_component_info() {
    let transformer = MergeTransformer::<i32>::new().with_name("test_merge".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_merge");
    assert_eq!(
      info.type_name,
      std::any::type_name::<MergeTransformer<i32>>()
    );
  }

  #[test]
  fn test_merge_transformer_default_name() {
    let transformer = MergeTransformer::<i32>::new();

    let info = transformer.component_info();
    assert_eq!(info.name, "merge_transformer");
  }

  #[test]
  fn test_merge_transformer_config_mut() {
    let mut transformer = MergeTransformer::<i32>::new();
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
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
    let _result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Streams should be consumed after transform
    assert_eq!(transformer.streams.len(), 0);
  }

  #[tokio::test]
  async fn test_merge_transformer_reuse() {
    let mut transformer = MergeTransformer::<i32>::new();

    // First use
    let input1 = stream::iter(vec![1, 2]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![1, 2]);

    // Add streams for second use
    let stream1 = create_stream(vec![3, 4]);
    transformer.add_stream(stream1);

    // Second use
    let input2 = stream::iter(vec![5, 6]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2.len(), 4);
    assert!(result2.contains(&3));
    assert!(result2.contains(&4));
    assert!(result2.contains(&5));
    assert!(result2.contains(&6));
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_merge_transformer_properties(
          name in ".*"
      ) {
          let transformer = MergeTransformer::<i32>::new()
              .with_name(name.clone());

          assert_eq!(transformer.config().name(), Some(name));
          assert_eq!(transformer.streams.len(), 0);
      }

      #[test]
      fn test_merge_transformer_error_strategies(
          retry_count in 0..10usize
      ) {
          let transformer = MergeTransformer::<i32>::new();

          let error = StreamError {
              source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "property test error")),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "property_test".to_string(),
                  component_type: "MergeTransformer".to_string(),
              },
              component: ComponentInfo {
                  name: "property_test".to_string(),
                  type_name: "MergeTransformer".to_string(),
              },
              retries: retry_count,
          };

          // Test different error strategies
          let transformer_skip = transformer.clone().with_error_strategy(ErrorStrategy::Skip);
          let transformer_retry = transformer.clone().with_error_strategy(ErrorStrategy::Retry(5));

          assert!(matches!(
              transformer_skip.handle_error(&error),
              ErrorAction::Skip
          ));

          if retry_count < 5 {
              assert!(matches!(
                  transformer_retry.handle_error(&error),
                  ErrorAction::Retry
              ));
          } else {
              assert!(matches!(
                  transformer_retry.handle_error(&error),
                  ErrorAction::Stop
              ));
          }
      }

      #[test]
      fn test_merge_transformer_config_persistence(
          name in ".*"
      ) {
          let transformer = MergeTransformer::<i32>::new()
              .with_name(name.clone())
              .with_error_strategy(ErrorStrategy::Skip);

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Skip
          ));
      }

      #[test]
      fn test_merge_transformer_stream_management(
          stream_count in 0..10usize
      ) {
          let mut transformer = MergeTransformer::<i32>::new();

          // Add streams
          for i in 0..stream_count {
              let stream = create_stream(vec![i as i32, (i + 1) as i32]);
              transformer.add_stream(stream);
          }

          assert_eq!(transformer.streams.len(), stream_count);

          // Just verify streams were added correctly
          assert_eq!(transformer.streams.len(), stream_count);
      }
  }

  #[tokio::test]
  async fn test_merge_transformer_edge_cases() {
    let mut transformer = MergeTransformer::<i32>::new();

    // Test with very large numbers of streams
    for i in 0..100 {
      let stream = create_stream(vec![i]);
      transformer.add_stream(stream);
    }

    assert_eq!(transformer.streams.len(), 100);

    let input = stream::iter(vec![999]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 101);
    assert!(result.contains(&999));

    // All streams should be consumed
    assert_eq!(transformer.streams.len(), 0);
  }

  #[tokio::test]
  async fn test_merge_transformer_different_data_types() {
    // Test with u64
    let mut transformer_u64 = MergeTransformer::<u64>::new();
    let stream1 = create_stream(vec![1u64, 2u64]);
    transformer_u64.add_stream(stream1);

    let input = stream::iter(vec![3u64]);
    let boxed_input = Box::pin(input);
    let result_u64: Vec<u64> = transformer_u64.transform(boxed_input).collect().await;
    assert_eq!(result_u64.len(), 3);

    // Test with f64
    let mut transformer_f64 = MergeTransformer::<f64>::new();
    let stream2 = create_stream(vec![1.5f64, 2.5f64]);
    transformer_f64.add_stream(stream2);

    let input_f64 = stream::iter(vec![3.5f64]);
    let boxed_input_f64 = Box::pin(input_f64);
    let result_f64: Vec<f64> = transformer_f64.transform(boxed_input_f64).collect().await;
    assert_eq!(result_f64.len(), 3);
  }
}
