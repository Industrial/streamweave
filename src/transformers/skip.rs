use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

#[derive(Clone)]
pub struct SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  skip: usize,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(skip: usize) -> Self {
    Self {
      skip,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Input for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let skip = self.skip;
    Box::pin(input.skip(skip))
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
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "skip_transformer".to_string()),
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
  async fn test_skip_basic() {
    let mut transformer = SkipTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![3, 4, 5]);
  }

  #[tokio::test]
  async fn test_skip_empty_input() {
    let mut transformer = SkipTransformer::new(2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = SkipTransformer::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  #[test]
  fn test_skip_transformer_new() {
    let transformer = SkipTransformer::<i32>::new(5);

    assert_eq!(transformer.skip, 5);
    assert_eq!(transformer.config().name(), None);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Stop
    ));
  }

  #[test]
  fn test_skip_transformer_with_error_strategy() {
    let transformer = SkipTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::Skip);

    assert_eq!(transformer.skip, 3);
    assert!(matches!(
      transformer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_skip_transformer_with_name() {
    let transformer = SkipTransformer::<i32>::new(4).with_name("test_skip".to_string());

    assert_eq!(transformer.skip, 4);
    assert_eq!(transformer.config().name(), Some("test_skip".to_string()));
  }

  #[tokio::test]
  async fn test_skip_transformer_skip_all() {
    let mut transformer = SkipTransformer::<i32>::new(5);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip all 5 elements
    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_skip_transformer_skip_more_than_input() {
    let mut transformer = SkipTransformer::<i32>::new(10);
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip more than available elements
    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_skip_transformer_skip_zero() {
    let mut transformer = SkipTransformer::<i32>::new(0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip nothing
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_skip_transformer_skip_one() {
    let mut transformer = SkipTransformer::<i32>::new(1);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip first element
    assert_eq!(result, vec![2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_skip_transformer_strings() {
    let mut transformer = SkipTransformer::<String>::new(2);
    let input = stream::iter(vec![
      "a".to_string(),
      "b".to_string(),
      "c".to_string(),
      "d".to_string(),
      "e".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(
      result,
      vec!["c".to_string(), "d".to_string(), "e".to_string()]
    );
  }

  #[tokio::test]
  async fn test_skip_transformer_different_types() {
    // Test with u64
    let mut transformer_u64 = SkipTransformer::<u64>::new(1);
    let input_u64 = stream::iter(vec![1u64, 2u64, 3u64, 4u64]);
    let boxed_input_u64 = Box::pin(input_u64);
    let result_u64: Vec<u64> = transformer_u64.transform(boxed_input_u64).collect().await;
    assert_eq!(result_u64, vec![2u64, 3u64, 4u64]);

    // Test with f64
    let mut transformer_f64 = SkipTransformer::<f64>::new(2);
    let input_f64 = stream::iter(vec![1.5f64, 2.5f64, 3.5f64, 4.5f64]);
    let boxed_input_f64 = Box::pin(input_f64);
    let result_f64: Vec<f64> = transformer_f64.transform(boxed_input_f64).collect().await;
    assert_eq!(result_f64, vec![3.5f64, 4.5f64]);
  }

  #[tokio::test]
  async fn test_skip_transformer_very_large_skip() {
    let mut transformer = SkipTransformer::<i32>::new(1000);
    let input = stream::iter((0..100).collect::<Vec<i32>>());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip more than available elements
    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_skip_transformer_error_handling() {
    let transformer = SkipTransformer::<i32>::new(3);

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "SkipTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "SkipTransformer".to_string(),
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
        component_type: "SkipTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "SkipTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      transformer.handle_error(&error),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_skip_transformer_error_context_creation() {
    let transformer = SkipTransformer::<i32>::new(2).with_name("test_skip".to_string());

    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_skip");
    assert_eq!(
      context.component_type,
      std::any::type_name::<SkipTransformer<i32>>()
    );
    assert_eq!(context.item, Some(42));
  }

  #[test]
  fn test_skip_transformer_component_info() {
    let transformer = SkipTransformer::<i32>::new(2).with_name("test_skip".to_string());

    let info = transformer.component_info();
    assert_eq!(info.name, "test_skip");
    assert_eq!(
      info.type_name,
      std::any::type_name::<SkipTransformer<i32>>()
    );
  }

  #[test]
  fn test_skip_transformer_default_name() {
    let transformer = SkipTransformer::<i32>::new(2);

    let info = transformer.component_info();
    assert_eq!(info.name, "skip_transformer");
  }

  #[test]
  fn test_skip_transformer_config_mut() {
    let mut transformer = SkipTransformer::<i32>::new(2);
    transformer.config_mut().name = Some("mutated_name".to_string());

    assert_eq!(
      transformer.config().name(),
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_skip_transformer_reuse() {
    let mut transformer = SkipTransformer::<i32>::new(2);

    // First use
    let input1 = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer.transform(boxed_input1).collect().await;
    assert_eq!(result1, vec![3, 4, 5]);

    // Second use
    let input2 = stream::iter(vec![10, 20, 30, 40, 50]);
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer.transform(boxed_input2).collect().await;
    assert_eq!(result2, vec![30, 40, 50]);
  }

  #[tokio::test]
  async fn test_skip_transformer_edge_cases() {
    let mut transformer = SkipTransformer::<i32>::new(1);

    // Test with single element
    let input = stream::iter(vec![42]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, Vec::<i32>::new());

    // Test with two elements
    let input = stream::iter(vec![42, 43]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![43]);
  }

  #[tokio::test]
  async fn test_skip_transformer_deterministic_behavior() {
    let mut transformer = SkipTransformer::<i32>::new(2);

    // Test that skipping is deterministic
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);

    let result1: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result1, vec![3, 4, 5, 6, 7, 8, 9, 10]);

    // Test again to ensure consistency
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);
    let result2: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result2, vec![3, 4, 5, 6, 7, 8, 9, 10]);
  }

  // Property-based tests using proptest
  proptest! {
    #[test]
    fn test_skip_transformer_properties(
      name in ".*"
    ) {
      let transformer = SkipTransformer::<i32>::new(5)
        .with_name(name.clone());

      assert_eq!(transformer.skip, 5);
      assert_eq!(transformer.config().name(), Some(name));
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Stop
      ));
    }

    #[test]
    fn test_skip_transformer_error_strategies(
      retry_count in 0..10usize
    ) {
      let transformer = SkipTransformer::<i32>::new(3);

      let error = StreamError {
        source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "property test error")),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "property_test".to_string(),
          component_type: "SkipTransformer".to_string(),
        },
        component: ComponentInfo {
          name: "property_test".to_string(),
          type_name: "SkipTransformer".to_string(),
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
    fn test_skip_transformer_config_persistence(
      name in ".*"
    ) {
      let transformer = SkipTransformer::<i32>::new(4)
        .with_name(name.clone())
        .with_error_strategy(ErrorStrategy::Skip);

      assert_eq!(transformer.skip, 4);
      assert_eq!(transformer.config().name(), Some(name));
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Skip
      ));
    }

    #[test]
    fn test_skip_transformer_skip_values(
      skip_count in 0..100usize
    ) {
      let transformer = SkipTransformer::<i32>::new(skip_count);
      assert_eq!(transformer.skip, skip_count);
      assert_eq!(transformer.config().name(), None);
      assert!(matches!(
        transformer.config().error_strategy(),
        ErrorStrategy::Stop
      ));
    }
  }

  #[tokio::test]
  async fn test_skip_transformer_stream_processing() {
    let mut transformer = SkipTransformer::<i32>::new(3);

    // Process a stream with various patterns
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let boxed_input = Box::pin(input);
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip first 3 elements
    assert_eq!(result, vec![4, 5, 6, 7, 8, 9, 10]);
  }

  #[tokio::test]
  async fn test_skip_transformer_nested_types() {
    let mut transformer = SkipTransformer::<Vec<i32>>::new(1);

    // Test skipping vectors
    let input = stream::iter(vec![vec![1, 2], vec![3, 4], vec![5, 6], vec![7, 8]]);
    let boxed_input = Box::pin(input);
    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![3, 4], vec![5, 6], vec![7, 8]]);
  }

  #[tokio::test]
  async fn test_skip_transformer_skip_exact_input_length() {
    let mut transformer = SkipTransformer::<i32>::new(5);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip exactly all elements
    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_skip_transformer_skip_partial() {
    let mut transformer = SkipTransformer::<i32>::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Should skip first 2 elements, keep last 3
    assert_eq!(result, vec![3, 4, 5]);
  }
}
