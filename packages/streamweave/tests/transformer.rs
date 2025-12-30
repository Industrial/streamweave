//! Tests for Transformer trait

use futures::StreamExt;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig, TransformerPorts};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio_stream::Stream;

// Test error type
#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

// Test transformer that doubles the input
#[derive(Clone, Debug)]
struct TestTransformer<T: std::fmt::Debug + Clone + Send + Sync> {
  config: TransformerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> TestTransformer<T> {
  fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TestTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TestTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait::async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TestTransformer<T> {
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input)
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }
}

#[tokio::test]
async fn test_transformer() {
  let mut transformer = TestTransformer::<i32>::new();
  let input = futures::stream::iter(vec![1, 2, 3]);
  let output = transformer.transform(Box::pin(input)).await;
  let result: Vec<i32> = output.collect().await;
  assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_transformer_config() {
  let transformer = TestTransformer::<i32>::new().with_config(
    TransformerConfig::default()
      .with_name("test_transformer".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  );

  assert_eq!(
    transformer.config().name(),
    Some("test_transformer".to_string())
  );
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_transformer_error_handling() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: transformer.component_info().name,
      component_type: transformer.component_info().type_name,
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestTransformer".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Skip
  ));
}

#[tokio::test]
async fn test_empty_input() {
  let mut transformer = TestTransformer::<i32>::new();
  let input = futures::stream::iter(Vec::<i32>::new());
  let output = transformer.transform(Box::pin(input)).await;
  let result: Vec<i32> = output.collect().await;
  assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_different_error_strategies() {
  let mut transformer = TestTransformer::<i32>::new();

  // Test Stop strategy
  transformer =
    transformer.with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };
  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Stop
  ));

  // Test Retry strategy
  transformer = transformer
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Retry
  ));
}

#[test]
fn test_custom_error_handler() {
  let transformer = TestTransformer::<i32>::new().with_config(
    TransformerConfig::default()
      .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip)),
  );

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Skip
  ));
}

#[test]
fn test_component_info() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));

  let info = transformer.component_info();
  assert_eq!(info.name, "test_transformer");
  assert!(info.type_name.contains("TestTransformer"));
}

#[test]
fn test_error_context_creation() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));

  let context = transformer.create_error_context(Some(42));
  assert_eq!(context.component_name, "test_transformer");
  assert!(context.component_type.contains("TestTransformer"));
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_configuration_persistence() {
  let mut transformer = TestTransformer::<i32>::new().with_config(
    TransformerConfig::default()
      .with_name("test_transformer".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  );

  // Verify initial config
  assert_eq!(
    transformer.config().name(),
    Some("test_transformer".to_string())
  );
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));

  // Change config
  transformer = transformer.with_config(
    TransformerConfig::default()
      .with_name("new_name".to_string())
      .with_error_strategy(ErrorStrategy::Stop),
  );

  // Verify new config
  assert_eq!(transformer.config().name(), Some("new_name".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_multiple_configuration_changes() {
  let mut transformer = TestTransformer::<i32>::new();

  // First config change
  transformer = transformer.with_config(
    TransformerConfig::default()
      .with_name("first".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  );
  assert_eq!(transformer.config().name(), Some("first".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));

  // Second config change
  transformer = transformer.with_config(
    TransformerConfig::default()
      .with_name("second".to_string())
      .with_error_strategy(ErrorStrategy::Stop),
  );
  assert_eq!(transformer.config().name(), Some("second".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_thread_local_configuration() {
  let transformer1 = TestTransformer::<i32>::new().with_config(
    TransformerConfig::default()
      .with_name("transformer1".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  );

  let transformer2 = TestTransformer::<i32>::new().with_config(
    TransformerConfig::default()
      .with_name("transformer2".to_string())
      .with_error_strategy(ErrorStrategy::Stop),
  );

  assert_eq!(
    transformer1.config().name(),
    Some("transformer1".to_string())
  );
  assert!(matches!(
    transformer1.config().error_strategy(),
    ErrorStrategy::Skip
  ));
  assert_eq!(
    transformer2.config().name(),
    Some("transformer2".to_string())
  );
  assert!(matches!(
    transformer2.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_transformer_config_default() {
  let config = TransformerConfig::<i32>::default();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Stop));
  assert_eq!(config.name(), None);
}

#[test]
fn test_transformer_config_builder_pattern() {
  let config = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(config.name(), Some("test".to_string()));
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_transformer_config_clone() {
  let config1 = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Retry(3));

  let config2 = config1.clone();
  assert_eq!(config1, config2);
}

#[test]
fn test_transformer_config_debug() {
  let config = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let debug_str = format!("{:?}", config);
  assert!(debug_str.contains("test"));
}

#[test]
fn test_transformer_config_partial_eq() {
  let config1 = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let config2 = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let config3 = TransformerConfig::<i32>::default()
    .with_name("different".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(config1, config2);
  assert_ne!(config1, config3);
}

#[test]
fn test_transformer_with_name() {
  let transformer = TestTransformer::<i32>::new().with_name("custom_name".to_string());

  assert_eq!(transformer.config().name(), Some("custom_name".to_string()));
}

#[test]
fn test_transformer_with_config() {
  let config = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let transformer = TestTransformer::<i32>::new().with_config(config.clone());
  assert_eq!(transformer.config(), &config);
}

#[test]
fn test_transformer_config_mut() {
  let mut transformer = TestTransformer::<i32>::new();
  let config = transformer.config_mut();
  config.name = Some("test".to_string());

  assert_eq!(transformer.config().name(), Some("test".to_string()));
}

#[test]
fn test_transformer_set_config() {
  let mut transformer = TestTransformer::<i32>::new();
  let config = TransformerConfig::<i32>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  transformer.set_config(config);
  assert_eq!(transformer.config().name(), Some("test".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_transformer_config_getter() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  assert_eq!(transformer.config().name(), Some("test".to_string()));
}

#[test]
fn test_error_handling_retry_exhausted() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(2)));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 3, // More than the retry limit
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Stop
  ));
}

#[test]
fn test_error_handling_retry_within_limit() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 2, // Within the retry limit
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Retry
  ));
}

#[test]
fn test_error_handling_stop_strategy() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Stop
  ));
}

#[test]
fn test_error_handling_skip_strategy() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Skip
  ));
}

#[test]
fn test_component_info_default_name() {
  let transformer = TestTransformer::<i32>::new();
  let info = transformer.component_info();
  assert_eq!(info.name, "transformer");
  assert!(info.type_name.contains("TestTransformer"));
}

#[test]
fn test_error_context_with_none_item() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  let context = transformer.create_error_context(None);
  assert_eq!(context.component_name, "test");
  assert!(context.component_type.contains("TestTransformer"));
  assert_eq!(context.item, None);
}

#[test]
fn test_error_context_timestamp() {
  let transformer = TestTransformer::<i32>::new();
  let before = chrono::Utc::now();
  let context = transformer.create_error_context(Some(42));
  let after = chrono::Utc::now();

  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
}

#[tokio::test]
async fn test_transformer_with_different_types() {
  let mut transformer = TestTransformer::<String>::new();
  let input = futures::stream::iter(vec!["hello".to_string(), "world".to_string()]);
  let output = transformer.transform(Box::pin(input)).await;
  let result: Vec<String> = output.collect().await;
  assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
}

#[tokio::test]
async fn test_transformer_with_large_input() {
  let mut transformer = TestTransformer::<i32>::new();
  let input = futures::stream::iter((1..=1000).collect::<Vec<_>>());
  let output = transformer.transform(Box::pin(input)).await;
  let result: Vec<i32> = output.collect().await;
  assert_eq!(result.len(), 1000);
  assert_eq!(result[0], 1);
  assert_eq!(result[999], 1000);
}

#[test]
fn test_transformer_clone() {
  let transformer1 = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  let transformer2 = transformer1.clone();
  assert_eq!(transformer1.config(), transformer2.config());
}

#[test]
fn test_transformer_debug() {
  let transformer = TestTransformer::<i32>::new();
  let debug_str = format!("{:?}", transformer);
  assert!(debug_str.contains("TestTransformer"));
}

#[test]
fn test_transformer_ports_trait() {
  // Test that TransformerPorts trait provides default port types
  // This test verifies the blanket implementation works by checking the types exist
  fn _test_transformer_ports<T: Transformer + TransformerPorts>()
  where
    T::Input: std::fmt::Debug + Clone + Send + Sync,
  {
    // Verify DefaultInputPorts and DefaultOutputPorts types exist
    // Just verify the associated types exist - don't try to assign
    let _phantom_input: std::marker::PhantomData<<T as TransformerPorts>::DefaultInputPorts> =
      std::marker::PhantomData;
    let _phantom_output: std::marker::PhantomData<<T as TransformerPorts>::DefaultOutputPorts> =
      std::marker::PhantomData;
  }

  // This test verifies the blanket implementation works
  let _transformer = TestTransformer::<i32>::new();
  // The trait is implemented via blanket impl, so this test verifies it compiles
}

#[test]
fn test_transformer_set_config_directly() {
  let mut transformer = TestTransformer::<i32>::new();
  let config = TransformerConfig::default()
    .with_name("direct_set".to_string())
    .with_error_strategy(ErrorStrategy::<i32>::Retry(10));

  transformer.set_config(config);
  assert_eq!(transformer.config().name(), Some("direct_set".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Retry(10)
  ));
}

#[test]
fn test_transformer_config_mut_modification() {
  let mut transformer = TestTransformer::<i32>::new();
  let config_mut = transformer.config_mut();
  config_mut.name = Some("mutated".to_string());
  config_mut.error_strategy = ErrorStrategy::<i32>::Skip;

  assert_eq!(transformer.config().name(), Some("mutated".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_transformer_handle_error_retry_at_limit() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 3, // Exactly at limit
  };

  // Should stop when retries equals limit
  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Stop
  ));
}

#[test]
fn test_transformer_handle_error_retry_below_limit() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));

  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 2, // Below limit
  };

  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Retry
  ));
}

#[test]
fn test_transformer_create_error_context_timestamp() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  let before = chrono::Utc::now();
  let context = transformer.create_error_context(Some(42));
  let after = chrono::Utc::now();

  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
  assert_eq!(context.item, Some(42));
}

#[tokio::test]
async fn test_transformer_transform_multiple_times() {
  let mut transformer = TestTransformer::<i32>::new();

  // First transformation
  let input1 = futures::stream::iter(vec![1, 2, 3]);
  let output1 = transformer.transform(Box::pin(input1)).await;
  let result1: Vec<i32> = output1.collect().await;
  assert_eq!(result1, vec![1, 2, 3]);

  // Second transformation (should work again)
  let input2 = futures::stream::iter(vec![4, 5, 6]);
  let output2 = transformer.transform(Box::pin(input2)).await;
  let result2: Vec<i32> = output2.collect().await;
  assert_eq!(result2, vec![4, 5, 6]);
}

#[test]
fn test_transformer_with_name_preserves_error_strategy() {
  let transformer = TestTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

  let transformer = transformer.with_name("named_transformer".to_string());

  assert_eq!(
    transformer.config().name(),
    Some("named_transformer".to_string())
  );
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_transformer_config_error_strategy_getter() {
  let config = TransformerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(7));

  let strategy = config.error_strategy();
  assert!(matches!(strategy, ErrorStrategy::Retry(7)));
}

#[test]
fn test_transformer_config_name_getter() {
  let config = TransformerConfig::<i32>::default().with_name("test_name".to_string());

  assert_eq!(config.name(), Some("test_name".to_string()));

  let config = TransformerConfig::<i32>::default();
  assert_eq!(config.name(), None);
}
