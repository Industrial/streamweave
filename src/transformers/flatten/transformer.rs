use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::flatten::flatten_transformer::FlattenTransformer;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Transformer for FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (Vec<T>,);
  type OutputPorts = (T,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.flat_map(futures::stream::iter))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
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
        .name()
        .clone()
        .unwrap_or_else(|| "flatten_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;
  use proptest::prelude::*;

  #[tokio::test]
  async fn test_empty_input_vectors() {
    let mut transformer = FlattenTransformer::new();
    let input = stream::iter(vec![vec![], vec![], vec![1], vec![]]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = FlattenTransformer::new()
      .with_error_strategy(ErrorStrategy::<Vec<i32>>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<Vec<i32>>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  #[test]
  fn test_flatten_transformer_new() {
    let transformer = FlattenTransformer::<i32>::new();

    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_flatten_transformer_with_error_strategy() {
    let transformer = FlattenTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_flatten_transformer_with_name() {
    let transformer = FlattenTransformer::<i32>::new().with_name("test_flatten".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("test_flatten".to_string())
    );
  }

  #[tokio::test]
  async fn test_flatten_transformer_basic() {
    let mut transformer = FlattenTransformer::<i32>::new();
    let input = stream::iter(vec![vec![1, 2], vec![3, 4], vec![5]]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_flatten_transformer_empty_input() {
    let mut transformer = FlattenTransformer::<i32>::new();
    let input = stream::iter(Vec::<Vec<i32>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_flatten_transformer_mixed_empty_vectors() {
    let mut transformer = FlattenTransformer::<i32>::new();
    let input = stream::iter(vec![vec![], vec![1, 2], vec![], vec![3], vec![]]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_flatten_transformer_single_element_vectors() {
    let mut transformer = FlattenTransformer::<i32>::new();
    let input = stream::iter(vec![vec![1], vec![2], vec![3]]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_flatten_transformer_large_vectors() {
    let mut transformer = FlattenTransformer::<i32>::new();
    let input = stream::iter(vec![
      vec![1, 2, 3, 4, 5],
      vec![6, 7, 8, 9, 10],
      vec![11, 12, 13, 14, 15],
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, (1..=15).collect::<Vec<i32>>());
  }

  #[tokio::test]
  async fn test_flatten_transformer_strings() {
    let mut transformer = FlattenTransformer::<String>::new();
    let input = stream::iter(vec![
      vec!["a".to_string(), "b".to_string()],
      vec!["c".to_string()],
      vec!["d".to_string(), "e".to_string(), "f".to_string()],
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec!["a", "b", "c", "d", "e", "f"]);
  }

  #[tokio::test]
  async fn test_flatten_transformer_different_types() {
    // Test with u64
    let mut transformer_u64 = FlattenTransformer::<u64>::new();
    let input_u64 = stream::iter(vec![vec![1u64, 2u64], vec![3u64]]);
    let boxed_input_u64 = Box::pin(input_u64);
    let result_u64: Vec<u64> = transformer_u64.transform(boxed_input_u64).collect().await;
    assert_eq!(result_u64, vec![1, 2, 3]);

    // Test with f64
    let mut transformer_f64 = FlattenTransformer::<f64>::new();
    let input_f64 = stream::iter(vec![vec![1.5f64, 2.5f64], vec![3.5f64]]);
    let boxed_input_f64 = Box::pin(input_f64);
    let result_f64: Vec<f64> = transformer_f64.transform(boxed_input_f64).collect().await;
    assert_eq!(result_f64, vec![1.5, 2.5, 3.5]);
  }

  #[test]
  fn test_flatten_transformer_error_handling() {
    let transformer = FlattenTransformer::<i32>::new();

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "FlattenTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "FlattenTransformer".to_string(),
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
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "FlattenTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "FlattenTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_flatten_transformer_error_context_creation() {
    let transformer = FlattenTransformer::<i32>::new().with_name("test_flatten".to_string());

    let context = transformer.create_error_context(Some(vec![1, 2, 3]));
    assert_eq!(context.component_name, "test_flatten");
    assert_eq!(
      context.component_type,
      std::any::type_name::<FlattenTransformer<i32>>()
    );
    assert_eq!(context.item, Some(vec![1, 2, 3]));
  }

  #[test]
  fn test_flatten_transformer_component_info() {
    let transformer = FlattenTransformer::<i32>::new().with_name("test_flatten".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_flatten");
    assert_eq!(
      info.type_name,
      std::any::type_name::<FlattenTransformer<i32>>()
    );
  }

  #[test]
  fn test_flatten_transformer_default_name() {
    let transformer = FlattenTransformer::<i32>::new();

    let info = transformer.component_info();
    assert_eq!(info.name, "flatten_transformer");
  }

  #[test]
  fn test_flatten_transformer_config_mut() {
    let mut transformer = FlattenTransformer::<i32>::new();
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_flatten_transformer_reuse() {
    let mut transformer = FlattenTransformer::<i32>::new();

    // First use
    let input1 = stream::iter(vec![vec![1, 2], vec![3]]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![1, 2, 3]);

    // Second use
    let input2 = stream::iter(vec![vec![4], vec![5, 6]]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![4, 5, 6]);
  }

  #[tokio::test]
  async fn test_flatten_transformer_edge_cases() {
    let mut transformer = FlattenTransformer::<i32>::new();

    // Test with very large vectors
    let large_vector: Vec<i32> = (0..1000).collect();
    let input = stream::iter(vec![large_vector.clone()]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, large_vector);

    // Test with many small vectors
    let many_small_vectors: Vec<Vec<i32>> = (0..100).map(|i| vec![i]).collect();
    let input2 = stream::iter(many_small_vectors);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, (0..100).collect::<Vec<i32>>());
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_flatten_transformer_properties(
          name in ".*"
      ) {
          let transformer = FlattenTransformer::<i32>::new()
              .with_name(name.clone());

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Stop
          ));
      }

      #[test]
      fn test_flatten_transformer_error_strategies(
          retry_count in 0..10usize
      ) {
          let transformer = FlattenTransformer::<i32>::new();

          let error = StreamError {
              source: Box::new(std::io::Error::other("property test error")),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "property_test".to_string(),
                  component_type: "FlattenTransformer".to_string(),
              },
              component: ComponentInfo {
                  name: "property_test".to_string(),
                  type_name: "FlattenTransformer".to_string(),
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
      fn test_flatten_transformer_config_persistence(
          name in ".*"
      ) {
          let transformer = FlattenTransformer::<i32>::new()
              .with_name(name.clone())
              .with_error_strategy(ErrorStrategy::Skip);

          assert_eq!(transformer.config().name(), Some(name));
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Skip
          ));
      }

      #[test]
      fn test_flatten_transformer_vector_processing(
          vector_count in 0..10usize,
          elements_per_vector in 0..10usize
      ) {
          // Create test data
          let mut expected_result = Vec::new();
          let mut input_vectors = Vec::new();

          for i in 0..vector_count {
              let mut vector = Vec::new();
              for j in 0..elements_per_vector {
                  let value = i * 100 + j;
                  vector.push(value);
                  expected_result.push(value);
              }
              input_vectors.push(vector);
          }

          // Test that the transformer can handle this data structure
          let transformer = FlattenTransformer::<i32>::new();
          assert_eq!(transformer.config().name(), None);
          assert!(matches!(
              transformer.config().error_strategy(),
              ErrorStrategy::Stop
          ));
      }
  }

  #[tokio::test]
  async fn test_flatten_transformer_stream_processing() {
    let mut transformer = FlattenTransformer::<i32>::new();

    // Process a stream with various vector patterns
    let input = stream::iter(vec![vec![1, 2], vec![], vec![3, 4, 5], vec![6], vec![]]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
  }

  #[tokio::test]
  async fn test_flatten_transformer_nested_empty_vectors() {
    let mut transformer = FlattenTransformer::<i32>::new();

    // Test with deeply nested empty vectors
    let input = stream::iter(vec![
      vec![],
      vec![],
      vec![],
      vec![1],
      vec![],
      vec![],
      vec![2, 3],
      vec![],
      vec![],
    ]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }
}
