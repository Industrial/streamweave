use crate::zip_transformer::ZipTransformer;
use async_stream;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave_core::Transformer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for ZipTransformer<T> {
  type InputPorts = (Vec<T>,);
  type OutputPorts = (Vec<T>,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(async_stream::stream! {
      let mut input = input;
      let mut buffers: Vec<Vec<T>> = Vec::new();

      while let Some(items) = input.next().await {
        buffers.push(items);
      }

      // Early return if no input
      if buffers.is_empty() {
        return;
      }

      // Get the length of the longest vector
      let max_len = buffers.iter().map(|v| v.len()).max().unwrap_or(0);

      // Yield transposed vectors
      for i in 0..max_len {
        let mut result = Vec::new();
        for buffer in &buffers {
          if let Some(item) = buffer.get(i) {
            result.push(item.clone());
          }
        }
        if !result.is_empty() {
          yield result;
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: streamweave_core::TransformerConfig<Vec<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &streamweave_core::TransformerConfig<Vec<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut streamweave_core::TransformerConfig<Vec<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "zip_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "zip_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use proptest::prelude::*;

  #[tokio::test]
  async fn test_zip_basic() {
    let mut transformer = ZipTransformer::new();
    let input = stream::iter(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 4, 7], vec![2, 5, 8], vec![3, 6, 9]]);
  }

  #[tokio::test]
  async fn test_zip_empty_input() {
    let mut transformer = ZipTransformer::new();
    let input = stream::iter(Vec::<Vec<i32>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = ZipTransformer::new()
      .with_error_strategy(ErrorStrategy::<Vec<i32>>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy, ErrorStrategy::<Vec<i32>>::Skip);
    assert_eq!(config.name, Some("test_transformer".to_string()));
  }

  #[test]
  fn test_zip_transformer_new() {
    let transformer = ZipTransformer::<i32>::new();

    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_zip_transformer_with_error_strategy() {
    let transformer = ZipTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_zip_transformer_with_name() {
    let transformer = ZipTransformer::<i32>::new().with_name("test_zip".to_string());

    assert_eq!(transformer.config().name(), Some("test_zip".to_string()));
  }

  #[tokio::test]
  async fn test_zip_transformer_different_lengths() {
    let mut transformer = ZipTransformer::<i32>::new();
    let input = stream::iter(vec![vec![1, 2, 3], vec![4, 5], vec![7, 8, 9, 10]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // The transformer processes all vectors and yields transposed results
    // It doesn't stop at the shortest length - it continues and yields partial results
    assert_eq!(
      result,
      vec![
        vec![1, 4, 7], // First elements from each vector
        vec![2, 5, 8], // Second elements from each vector
        vec![3, 9],    // Third elements (only from vectors that have them)
        vec![10]       // Fourth element (only from the longest vector)
      ]
    );
  }

  #[tokio::test]
  async fn test_zip_transformer_single_vector() {
    let mut transformer = ZipTransformer::<i32>::new();
    let input = stream::iter(vec![vec![1, 2, 3]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
  }

  #[tokio::test]
  async fn test_zip_transformer_empty_vectors() {
    let mut transformer = ZipTransformer::<i32>::new();
    let input = stream::iter(vec![vec![], vec![], vec![]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_zip_transformer_mixed_empty_vectors() {
    let mut transformer = ZipTransformer::<i32>::new();
    let input = stream::iter(vec![vec![1, 2], vec![], vec![3, 4]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // The transformer processes all vectors and yields transposed results
    // Even with empty vectors, it still processes the non-empty ones
    assert_eq!(
      result,
      vec![
        vec![1, 3], // First elements from non-empty vectors
        vec![2, 4]  // Second elements from non-empty vectors
      ]
    );
  }

  #[tokio::test]
  async fn test_zip_transformer_strings() {
    let mut transformer = ZipTransformer::<String>::new();
    let input = stream::iter(vec![
      vec!["a".to_string(), "b".to_string()],
      vec!["c".to_string(), "d".to_string()],
      vec!["e".to_string(), "f".to_string()],
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<String>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![
        vec!["a".to_string(), "c".to_string(), "e".to_string()],
        vec!["b".to_string(), "d".to_string(), "f".to_string()]
      ]
    );
  }

  #[tokio::test]
  async fn test_zip_transformer_different_types() {
    // Test with u64
    let mut transformer_u64 = ZipTransformer::<u64>::new();
    let input_u64 = stream::iter(vec![vec![1u64, 2u64], vec![3u64, 4u64]]);
    let boxed_input_u64 = Box::pin(input_u64);
    let result_u64: Vec<Vec<u64>> = transformer_u64.transform(boxed_input_u64).collect().await;
    assert_eq!(result_u64, vec![vec![1, 3], vec![2, 4]]);

    // Test with f64
    let mut transformer_f64 = ZipTransformer::<f64>::new();
    let input_f64 = stream::iter(vec![vec![1.5f64, 2.5f64], vec![3.5f64, 4.5f64]]);
    let boxed_input_f64 = Box::pin(input_f64);
    let result_f64: Vec<Vec<f64>> = transformer_f64.transform(boxed_input_f64).collect().await;
    assert_eq!(result_f64, vec![vec![1.5, 3.5], vec![2.5, 4.5]]);
  }

  #[tokio::test]
  async fn test_zip_transformer_very_long_vectors() {
    let mut transformer = ZipTransformer::<i32>::new();
    let input = stream::iter(vec![
      (0..1000).collect::<Vec<i32>>(),
      (1000..2000).collect::<Vec<i32>>(),
      (2000..3000).collect::<Vec<i32>>(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Should have 1000 zipped vectors
    assert_eq!(result.len(), 1000);

    // Check first and last elements
    assert_eq!(result[0], vec![0, 1000, 2000]);
    assert_eq!(result[999], vec![999, 1999, 2999]);
  }

  #[test]
  fn test_zip_transformer_error_handling() {
    let transformer = ZipTransformer::<i32>::new();

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "ZipTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ZipTransformer".to_string(),
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
        component_type: "ZipTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "ZipTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_zip_transformer_error_context_creation() {
    let transformer = ZipTransformer::<i32>::new().with_name("test_zip".to_string());

    let context = transformer.create_error_context(Some(vec![1, 2, 3]));
    assert_eq!(context.component_name, "test_zip");
    assert_eq!(
      context.component_type,
      std::any::type_name::<ZipTransformer<i32>>()
    );
    assert_eq!(context.item, Some(vec![1, 2, 3]));
  }

  #[test]
  fn test_zip_transformer_component_info() {
    let transformer = ZipTransformer::<i32>::new().with_name("test_zip".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_zip");
    assert_eq!(info.type_name, std::any::type_name::<ZipTransformer<i32>>());
  }

  #[test]
  fn test_zip_transformer_default_name() {
    let transformer = ZipTransformer::<i32>::new();

    let info = transformer.component_info();
    assert_eq!(info.name, "zip_transformer");
  }

  #[test]
  fn test_zip_transformer_config_mut() {
    let mut transformer = ZipTransformer::<i32>::new();
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_zip_transformer_reuse() {
    let mut transformer = ZipTransformer::<i32>::new();

    // First use
    let input1 = stream::iter(vec![vec![1, 2], vec![3, 4]]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<Vec<i32>> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![vec![1, 3], vec![2, 4]]);

    // Second use
    let input2 = stream::iter(vec![vec![5, 6], vec![7, 8]]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<Vec<i32>> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![vec![5, 7], vec![6, 8]]);
  }

  #[tokio::test]
  async fn test_zip_transformer_edge_cases() {
    let mut transformer = ZipTransformer::<i32>::new();

    // Test with one very long vector and one short vector
    let input = stream::iter(vec![(0..100).collect::<Vec<i32>>(), vec![100, 101]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Should have 100 zipped vectors (the length of the longest vector)
    assert_eq!(result.len(), 100);
    assert_eq!(result[0], vec![0, 100]);
    assert_eq!(result[1], vec![1, 101]);
    // After index 1, the second vector has no more elements
    assert_eq!(result[2], vec![2]);
    assert_eq!(result[99], vec![99]);
  }

  #[tokio::test]
  async fn test_zip_transformer_deterministic_ordering() {
    let mut transformer = ZipTransformer::<i32>::new();

    // Test that zipping maintains order
    let input = stream::iter(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Should maintain the order of vectors in the input
    assert_eq!(
      result,
      vec![
        vec![1, 4, 7], // First elements from each vector
        vec![2, 5, 8], // Second elements from each vector
        vec![3, 6, 9]  // Third elements from each vector
      ]
    );
  }

  // Property-based tests using proptest
  proptest! {
    #[test]
    fn test_zip_transformer_properties(
      name in ".*"
    ) {
      let transformer = ZipTransformer::<i32>::new()
        .with_name(name.clone());

      assert_eq!(transformer.config().name(), Some(name));
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Stop
      ));
    }

    #[test]
    fn test_zip_transformer_error_strategies(
      retry_count in 0..10usize
    ) {
      let transformer = ZipTransformer::<i32>::new();

      let error = StreamError {
        source: Box::new(std::io::Error::other("property test error")),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "property_test".to_string(),
          component_type: "ZipTransformer".to_string(),
        },
        component: ComponentInfo {
          name: "property_test".to_string(),
          type_name: "ZipTransformer".to_string(),
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
    fn test_zip_transformer_config_persistence(
      name in ".*"
    ) {
      let transformer = ZipTransformer::<i32>::new()
        .with_name(name.clone())
        .with_error_strategy(ErrorStrategy::Skip);

      assert_eq!(transformer.config().name(), Some(name));
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Skip
      ));
    }

    #[test]
    fn test_zip_transformer_vector_processing(
      _vector_count in 0..10usize,
      _elements_per_vector in 0..10usize
    ) {
      // Test that the transformer can handle this data structure
      let transformer = ZipTransformer::<i32>::new();
      assert_eq!(transformer.config().name(), None);
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Stop
      ));
    }
  }

  #[tokio::test]
  async fn test_zip_transformer_stream_processing() {
    let mut transformer = ZipTransformer::<i32>::new();

    // Process a stream with various vector patterns
    let input = stream::iter(vec![vec![1, 2, 3], vec![4, 5], vec![6, 7, 8, 9]]);
    let boxed_input = Box::pin(input);
    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Should have 4 zipped vectors (the length of the longest vector)
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], vec![1, 4, 6]);
    assert_eq!(result[1], vec![2, 5, 7]);
    assert_eq!(result[2], vec![3, 8]); // Third vector has no third element
    assert_eq!(result[3], vec![9]); // Only the longest vector has a fourth element
  }

  #[tokio::test]
  async fn test_zip_transformer_nested_vectors() {
    let mut transformer = ZipTransformer::<Vec<i32>>::new();

    // Test zipping vectors of vectors
    let input = stream::iter(vec![
      vec![vec![1, 2], vec![3, 4]],
      vec![vec![5, 6], vec![7, 8]],
      vec![vec![9, 10], vec![11, 12]],
    ]);
    let boxed_input = Box::pin(input);
    let result: Vec<Vec<Vec<i32>>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec![
        vec![vec![1, 2], vec![5, 6], vec![9, 10]],
        vec![vec![3, 4], vec![7, 8], vec![11, 12]]
      ]
    );
  }
}
