//! Tests for Transformer trait with Message<T>

use futures::StreamExt;
use std::pin::Pin;
use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};
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

// Test transformer that doubles the payload while preserving message structure
#[derive(Clone, Debug)]
struct TestTransformer<T: std::fmt::Debug + Clone + Send + Sync> {
  config: TransformerConfig<Message<T>>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> TestTransformer<T> {
  fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TestTransformer<T> {
  type Input = Message<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TestTransformer<T> {
  type Output = Message<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

#[async_trait::async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TestTransformer<T>
where
  T: std::ops::Mul<Output = T> + From<u8> + Copy,
{
  type InputPorts = (Message<T>,);
  type OutputPorts = (Message<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|msg| {
      // Double the payload while preserving message ID and metadata
      let doubled_payload = *msg.payload() * T::from(2);
      msg.with_payload(doubled_payload)
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<T>> {
    &mut self.config
  }
}

// Simple pass-through transformer for non-numeric types
#[derive(Clone, Debug)]
struct PassThroughTransformer<T: std::fmt::Debug + Clone + Send + Sync> {
  config: TransformerConfig<Message<T>>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> PassThroughTransformer<T> {
  fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for PassThroughTransformer<T> {
  type Input = Message<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for PassThroughTransformer<T> {
  type Output = Message<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

#[async_trait::async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for PassThroughTransformer<T> {
  type InputPorts = (Message<T>,);
  type OutputPorts = (Message<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input) // Pass through unchanged
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<T>> {
    &mut self.config
  }
}

#[tokio::test]
async fn test_transformer_with_messages() {
  let mut transformer = TestTransformer::<i32>::new();
  let input = futures::stream::iter(vec![wrap_message(1), wrap_message(2), wrap_message(3)]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  assert_eq!(messages.len(), 3);
  assert_eq!(*messages[0].payload(), 2); // Doubled
  assert_eq!(*messages[1].payload(), 4); // Doubled
  assert_eq!(*messages[2].payload(), 6); // Doubled
}

#[tokio::test]
async fn test_transformer_preserves_message_ids() {
  let mut transformer = TestTransformer::<i32>::new();
  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(2);
  let id3 = MessageId::new_sequence(3);

  let input = futures::stream::iter(vec![
    Message::new(1, id1.clone()),
    Message::new(2, id2.clone()),
    Message::new(3, id3.clone()),
  ]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  // Verify IDs are preserved
  assert_eq!(*messages[0].id(), id1);
  assert_eq!(*messages[1].id(), id2);
  assert_eq!(*messages[2].id(), id3);
}

#[tokio::test]
async fn test_transformer_preserves_metadata() {
  let mut transformer = TestTransformer::<i32>::new();
  let metadata = MessageMetadata::new().source("test-source");

  let input = futures::stream::iter(vec![
    Message::with_metadata(1, MessageId::new_uuid(), metadata.clone()),
    Message::with_metadata(2, MessageId::new_uuid(), metadata.clone()),
  ]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  // Verify metadata is preserved
  for msg in &messages {
    assert_eq!(msg.metadata().get_source(), metadata.get_source());
  }
}

#[tokio::test]
async fn test_transformer_payload_access() {
  let mut transformer = TestTransformer::<i32>::new();
  let input = futures::stream::iter(vec![wrap_message(5), wrap_message(10)]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  // Extract payloads
  let payloads: Vec<i32> = messages.iter().map(|m| *m.payload()).collect();
  assert_eq!(payloads, vec![10, 20]);
}

#[test]
fn test_transformer_config() {
  let transformer = PassThroughTransformer::<i32>::new().with_config(
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
fn test_transformer_error_handling_with_messages() {
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: transformer.component_info().name,
      component_type: transformer.component_info().type_name,
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "PassThroughTransformer".to_string(),
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
  let mut transformer = PassThroughTransformer::<i32>::new();
  let input = futures::stream::iter(Vec::<Message<i32>>::new());
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;
  assert!(messages.is_empty());
}

#[test]
fn test_different_error_strategies() {
  let mut transformer = PassThroughTransformer::<i32>::new();

  // Test Stop strategy
  transformer =
    transformer.with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
    component: ComponentInfo::default(),
    retries: 0,
  };
  assert!(matches!(
    transformer.handle_error(&error),
    ErrorAction::Stop
  ));

  // Test Retry strategy
  let msg2 = wrap_message(43);
  transformer = transformer
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  let error2 = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg2),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
    component: ComponentInfo::default(),
    retries: 0,
  };
  assert!(matches!(
    transformer.handle_error(&error2),
    ErrorAction::Retry
  ));
}

#[test]
fn test_custom_error_handler() {
  let transformer = PassThroughTransformer::<i32>::new().with_config(
    TransformerConfig::default()
      .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip)),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));

  let info = transformer.component_info();
  assert_eq!(info.name, "test_transformer");
  assert!(info.type_name.contains("PassThroughTransformer"));
}

#[test]
fn test_error_context_creation_with_message() {
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));
  let msg = wrap_message(42);

  let context = transformer.create_error_context(Some(msg.clone()));
  assert_eq!(context.component_name, "test_transformer");
  assert!(context.component_type.contains("PassThroughTransformer"));
  assert_eq!(context.item, Some(msg));

  // Verify message ID is accessible from error context
  if let Some(error_msg) = &context.item {
    assert!(error_msg.id().is_uuid());
    assert_eq!(*error_msg.payload(), 42);
  }
}

#[test]
fn test_error_context_message_metadata() {
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test_transformer".to_string()));
  let metadata = MessageMetadata::new().source("test-source");
  let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata);

  let context = transformer.create_error_context(Some(msg.clone()));

  // Verify message metadata is accessible from error context
  if let Some(error_msg) = &context.item {
    assert_eq!(error_msg.metadata().get_source(), Some("test-source"));
  }
}

#[test]
fn test_configuration_persistence() {
  let mut transformer = PassThroughTransformer::<i32>::new().with_config(
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
fn test_transformer_config_default() {
  let config = TransformerConfig::<Message<i32>>::default();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Stop));
  assert_eq!(config.name(), None);
}

#[test]
fn test_transformer_config_builder_pattern() {
  let config = TransformerConfig::<Message<i32>>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(config.name(), Some("test".to_string()));
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_transformer_config_clone() {
  let config1 = TransformerConfig::<Message<i32>>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Retry(3));

  let config2 = config1.clone();
  assert_eq!(config1, config2);
}

#[test]
fn test_transformer_config_debug() {
  let config = TransformerConfig::<Message<i32>>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let debug_str = format!("{:?}", config);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_transformer_config_partial_eq() {
  let config1 = TransformerConfig::<Message<i32>>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let config2 = TransformerConfig::<Message<i32>>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let config3 = TransformerConfig::<Message<i32>>::default()
    .with_name("different".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(config1, config2);
  assert_ne!(config1, config3);
}

#[test]
fn test_transformer_with_name() {
  let transformer = PassThroughTransformer::<i32>::new().with_name("custom_name".to_string());

  assert_eq!(transformer.config().name(), Some("custom_name".to_string()));
}

#[test]
fn test_transformer_with_config() {
  let config = TransformerConfig::<Message<i32>>::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let transformer = PassThroughTransformer::<i32>::new().with_config(config.clone());
  assert_eq!(transformer.config(), &config);
}

#[test]
fn test_transformer_config_mut() {
  let mut transformer = PassThroughTransformer::<i32>::new();
  let config = transformer.config_mut();
  config.name = Some("test".to_string());

  assert_eq!(transformer.config().name(), Some("test".to_string()));
}

#[test]
fn test_transformer_set_config() {
  let mut transformer = PassThroughTransformer::<i32>::new();
  let config = TransformerConfig::<Message<i32>>::default()
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  assert_eq!(transformer.config().name(), Some("test".to_string()));
}

#[test]
fn test_error_handling_retry_exhausted() {
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(2)));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new();
  let info = transformer.component_info();
  assert_eq!(info.name, "transformer");
  assert!(info.type_name.contains("PassThroughTransformer"));
}

#[test]
fn test_error_context_with_none_item() {
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  let context = transformer.create_error_context(None);
  assert_eq!(context.component_name, "test");
  assert!(context.component_type.contains("PassThroughTransformer"));
  assert_eq!(context.item, None);
}

#[test]
fn test_error_context_timestamp() {
  let transformer = PassThroughTransformer::<i32>::new();
  let msg = wrap_message(42);
  let before = chrono::Utc::now();
  let context = transformer.create_error_context(Some(msg.clone()));
  let after = chrono::Utc::now();

  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
  assert_eq!(context.item, Some(msg));
}

#[tokio::test]
async fn test_transformer_with_different_types() {
  let mut transformer = PassThroughTransformer::<String>::new();
  let input = futures::stream::iter(vec![
    wrap_message("hello".to_string()),
    wrap_message("world".to_string()),
  ]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<String>> = output.collect().await;

  assert_eq!(messages.len(), 2);
  assert_eq!(*messages[0].payload(), "hello".to_string());
  assert_eq!(*messages[1].payload(), "world".to_string());
}

#[tokio::test]
async fn test_transformer_with_large_input() {
  let mut transformer = PassThroughTransformer::<i32>::new();
  let input = futures::stream::iter((1..=1000).map(wrap_message).collect::<Vec<_>>());
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  assert_eq!(messages.len(), 1000);
  assert_eq!(*messages[0].payload(), 1);
  assert_eq!(*messages[999].payload(), 1000);
}

#[test]
fn test_transformer_clone() {
  let transformer1 = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));

  let transformer2 = transformer1.clone();
  assert_eq!(transformer1.config(), transformer2.config());
}

#[test]
fn test_transformer_debug() {
  let transformer = PassThroughTransformer::<i32>::new();
  let debug_str = format!("{:?}", transformer);
  assert!(debug_str.contains("PassThroughTransformer"));
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
  let _transformer = PassThroughTransformer::<i32>::new();
  // The trait is implemented via blanket impl, so this test verifies it compiles
}

#[test]
fn test_transformer_set_config_directly() {
  let mut transformer = PassThroughTransformer::<i32>::new();
  let config = TransformerConfig::default()
    .with_name("direct_set".to_string())
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(10));

  transformer.set_config(config);
  assert_eq!(transformer.config().name(), Some("direct_set".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Retry(10)
  ));
}

#[test]
fn test_transformer_config_mut_modification() {
  let mut transformer = PassThroughTransformer::<i32>::new();
  let config_mut = transformer.config_mut();
  config_mut.name = Some("mutated".to_string());
  config_mut.error_strategy = ErrorStrategy::<Message<i32>>::Skip;

  assert_eq!(transformer.config().name(), Some("mutated".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_transformer_handle_error_retry_at_limit() {
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestTransformer".to_string(),
    },
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
  let transformer = PassThroughTransformer::<i32>::new()
    .with_config(TransformerConfig::default().with_name("test".to_string()));
  let msg = wrap_message(42);

  let before = chrono::Utc::now();
  let context = transformer.create_error_context(Some(msg.clone()));
  let after = chrono::Utc::now();

  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
  assert_eq!(context.item, Some(msg));
}

#[tokio::test]
async fn test_transformer_transform_multiple_times() {
  let mut transformer = PassThroughTransformer::<i32>::new();

  // First transformation
  let input1 = futures::stream::iter(vec![wrap_message(1), wrap_message(2), wrap_message(3)]);
  let output1 = transformer.transform(Box::pin(input1)).await;
  let messages1: Vec<Message<i32>> = output1.collect().await;
  assert_eq!(messages1.len(), 3);
  assert_eq!(*messages1[0].payload(), 1);

  // Second transformation (should work again)
  let input2 = futures::stream::iter(vec![wrap_message(4), wrap_message(5), wrap_message(6)]);
  let output2 = transformer.transform(Box::pin(input2)).await;
  let messages2: Vec<Message<i32>> = output2.collect().await;
  assert_eq!(messages2.len(), 3);
  assert_eq!(*messages2[0].payload(), 4);
}

#[test]
fn test_transformer_with_name_preserves_error_strategy() {
  let transformer = PassThroughTransformer::<i32>::new()
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
  let config =
    TransformerConfig::<Message<i32>>::default().with_error_strategy(ErrorStrategy::Retry(7));

  let strategy = config.error_strategy();
  assert!(matches!(strategy, ErrorStrategy::Retry(7)));
}

#[test]
fn test_transformer_config_name_getter() {
  let config = TransformerConfig::<Message<i32>>::default().with_name("test_name".to_string());

  assert_eq!(config.name(), Some("test_name".to_string()));

  let config = TransformerConfig::<Message<i32>>::default();
  assert_eq!(config.name(), None);
}

#[tokio::test]
async fn test_transformer_message_transformation() {
  let mut transformer = TestTransformer::<i32>::new();
  let input = futures::stream::iter(vec![wrap_message(5), wrap_message(10), wrap_message(15)]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  // Verify transformation (doubling)
  assert_eq!(*messages[0].payload(), 10);
  assert_eq!(*messages[1].payload(), 20);
  assert_eq!(*messages[2].payload(), 30);
}

#[tokio::test]
async fn test_transformer_metadata_preservation_through_transformation() {
  let mut transformer = TestTransformer::<i32>::new();
  let metadata = MessageMetadata::new()
    .source("test-source")
    .partition(0)
    .offset(100);

  let input = futures::stream::iter(vec![
    Message::with_metadata(5, MessageId::new_uuid(), metadata.clone()),
    Message::with_metadata(10, MessageId::new_uuid(), metadata.clone()),
  ]);
  let output = transformer.transform(Box::pin(input)).await;
  let messages: Vec<Message<i32>> = output.collect().await;

  // Verify metadata is preserved through transformation
  for msg in &messages {
    assert_eq!(msg.metadata().get_source(), metadata.get_source());
    assert_eq!(msg.metadata().partition, metadata.partition);
    assert_eq!(msg.metadata().offset, metadata.offset);
  }
}
