//! # Transformer Trait Test Suite
//!
//! Comprehensive test suite for the `Transformer` trait, including configuration,
//! error handling, component information, and transformation operations.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **TransformerConfig**: Default values, error strategy, name configuration, and builder pattern
//! - **Transformer Methods**: set_config, config, config_mut, with_name, with_config
//! - **Error Handling**: All error strategy variants (Stop, Skip, Retry, Custom) and retry logic
//! - **Component Info**: Component name and type information retrieval
//! - **Error Context**: Creation of error contexts with item information
//! - **Transformation**: Stream transformation operations and port type handling
//! - **TransformerPorts**: Default input and output port type assignments
//!
//! ## Test Organization
//!
//! Tests are organized to cover:
//!
//! 1. **Configuration Tests**: Config creation, modification, and access
//! 2. **Error Handling Tests**: All error strategy branches and retry logic
//! 3. **Component Info Tests**: Name and type information retrieval
//! 4. **Transformation Tests**: Stream transformation operations
//! 5. **Port Tests**: Port type assignments and compatibility
//! 6. **Internal Method Tests**: Verification of trait default implementations
//!
//! ## Key Concepts
//!
//! - **Transformer**: A component that transforms input streams into output streams
//! - **TransformerConfig**: Configuration for transformers including error strategy and name
//! - **Error Strategy**: How to handle errors during transformation (Stop, Skip, Retry, Custom)
//! - **TransformerPorts**: Type-level port definitions for input and output
//!
//! ## Usage
//!
//! These tests ensure that transformers correctly transform streams, handle errors,
//! and provide proper component information for debugging and error reporting.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::MapTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::pin::Pin;

#[test]
fn test_transformer_config_default() {
  let config = TransformerConfig::<i32>::default();

  assert_eq!(config.name(), None);
  match config.error_strategy() {
    ErrorStrategy::Stop => {} // Expected
    _ => panic!("Expected Stop strategy by default"),
  }
}

#[test]
fn test_transformer_config_with_error_strategy() {
  let config = TransformerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);

  match config.error_strategy() {
    ErrorStrategy::Skip => {} // Expected
    _ => panic!("Expected Skip strategy"),
  }
}

#[test]
fn test_transformer_config_with_name() {
  let config = TransformerConfig::<i32>::default().with_name("my_transformer".to_string());

  assert_eq!(config.name(), Some("my_transformer".to_string()));
}

#[test]
fn test_transformer_config_builder_chain() {
  let config = TransformerConfig::<i32>::default()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .with_name("test_transformer".to_string());

  assert_eq!(config.name(), Some("test_transformer".to_string()));
  match config.error_strategy() {
    ErrorStrategy::Retry(3) => {}
    _ => panic!("Expected Retry(3) strategy"),
  }
}

#[test]
fn test_transformer_config_error_strategy_getter() {
  let config = TransformerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);

  let strategy = config.error_strategy();
  match strategy {
    ErrorStrategy::Skip => {}
    _ => panic!("Expected Skip strategy"),
  }
}

#[tokio::test]
async fn test_transformer_set_config() {
  let mut transformer = MapTransformer::<_, i32, i32>::new(|x| x * 2);
  let config = TransformerConfig::<i32>::default().with_name("new_name".to_string());

  transformer.set_config(config);
  assert_eq!(transformer.config().name(), Some("new_name".to_string()));
}

#[test]
fn test_transformer_config_getter() {
  let transformer = MapTransformer::<_, i32, i32>::new(|x| x * 2);
  let config = transformer.config();
  assert_eq!(config.name(), None);
}

#[tokio::test]
async fn test_transformer_config_mut() {
  let mut transformer = MapTransformer::<_, i32, i32>::new(|x| x * 2);
  transformer.config_mut().name = Some("modified".to_string());
  assert_eq!(transformer.config().name(), Some("modified".to_string()));
}

#[test]
fn test_transformer_with_name() {
  let transformer = MapTransformer::<_, i32, i32>::new(|x| x * 2);
  let named = transformer.with_name("named_transformer".to_string());
  assert_eq!(named.config().name(), Some("named_transformer".to_string()));
}

#[test]
fn test_transformer_handle_error_stop() {
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test error"),
    context,
    component,
    retries: 0,
  };

  // MapTransformer overrides handle_error, so test through TestCloneTransformer
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let action = Transformer::handle_error(&test_transformer, &error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_transformer_handle_error_skip() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };

  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test error"),
    context,
    component,
    retries: 0,
  };

  let action = Transformer::handle_error(&test_transformer, &error);
  assert!(matches!(action, ErrorAction::Skip));
}

#[test]
fn test_transformer_handle_error_retry() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)),
  };

  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test error"),
    context,
    component,
    retries: 2, // Less than 3, should retry
  };

  let action = Transformer::handle_error(&test_transformer, &error);
  assert!(matches!(action, ErrorAction::Retry));
}

#[test]
fn test_transformer_handle_error_retry_exceeded() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)),
  };

  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test error"),
    context,
    component,
    retries: 3, // Equal to limit, should stop
  };

  let action = Transformer::handle_error(&test_transformer, &error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_transformer_component_info() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let info = Transformer::component_info(&test_transformer);
  assert_eq!(info.name, "transformer"); // Default name when None
  assert!(info.type_name.contains("TestCloneTransformer"));
}

#[test]
fn test_transformer_component_info_with_name() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("named_transformer".to_string()),
  };
  let info = Transformer::component_info(&test_transformer);
  assert_eq!(info.name, "named_transformer");
  assert!(info.type_name.contains("TestCloneTransformer"));
}

#[test]
fn test_transformer_create_error_context() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("test_transformer".to_string()),
  };
  let context = Transformer::create_error_context(&test_transformer, Some(42));

  assert_eq!(context.component_name, "test_transformer");
  assert!(context.component_type.contains("TestCloneTransformer"));
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_transformer_create_error_context_no_item() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let context = Transformer::create_error_context(&test_transformer, None);

  assert_eq!(context.item, None);
  assert!(!context.component_type.is_empty());
}

#[tokio::test]
async fn test_transformer_transform() {
  let mut transformer = MapTransformer::<_, i32, i32>::new(|x| x * 2);
  let input = futures::stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let mut output = transformer.transform(boxed_input).await;

  let mut collected = Vec::new();
  while let Some(item) = output.next().await {
    collected.push(item);
  }

  assert_eq!(collected, vec![2, 4, 6]);
}

#[test]
fn test_transformer_ports_trait() {
  // Test that MapTransformer implements TransformerPorts correctly
  use crate::port::PortList;
  use crate::transformer::TransformerPorts;

  fn assert_ports<T>()
  where
    T: Transformer + TransformerPorts,
    T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <T as TransformerPorts>::DefaultInputPorts: PortList,
    <T as TransformerPorts>::DefaultOutputPorts: PortList,
  {
    // Just checking it compiles
  }

  assert_ports::<MapTransformer<fn(i32) -> i32, i32, i32>>();
}

// Test with_config method - requires Clone
// We use a single type parameter T for both Input and Output (identity transform)
#[derive(Clone)]
struct TestCloneTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  config: TransformerConfig<T>,
}

impl<T> Input for TestCloneTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TestCloneTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for TestCloneTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, mut input: Self::InputStream) -> Self::OutputStream {
    // Identity transform for testing
    Box::pin(async_stream::stream! {
      while let Some(item) = input.next().await {
        yield item;
      }
    })
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
}

#[test]
fn test_transformer_with_config() {
  let transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("original".to_string()),
  };

  let new_config = TransformerConfig::default()
    .with_name("new_name".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let configured = transformer.with_config(new_config);
  assert_eq!(configured.config().name(), Some("new_name".to_string()));
  assert!(matches!(
    configured.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test TransformerPorts trait more directly
#[test]
fn test_transformer_ports_default_ports() {
  use crate::port::PortList;
  use crate::transformer::TransformerPorts;

  // Test that the blanket implementation works
  fn check_ports_type<T>()
  where
    T: Transformer + TransformerPorts,
    T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <T as TransformerPorts>::DefaultInputPorts: PortList,
    <T as TransformerPorts>::DefaultOutputPorts: PortList,
  {
    // Just verifying it compiles - the type constraint validates the impl
  }

  check_ports_type::<MapTransformer<fn(i32) -> i32, i32, i32>>();

  // Verify the types are correct
  type MapInputPorts =
    <MapTransformer<fn(i32) -> i32, i32, i32> as TransformerPorts>::DefaultInputPorts;
  type MapOutputPorts =
    <MapTransformer<fn(i32) -> i32, i32, i32> as TransformerPorts>::DefaultOutputPorts;

  fn assert_type<T: PortList>() {}
  assert_type::<MapInputPorts>();
  assert_type::<MapOutputPorts>();
  assert_type::<(i32,)>();
}

// Test component_info with empty name (should use default)
#[test]
fn test_transformer_component_info_empty_name() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let info = Transformer::component_info(&test_transformer);
  assert_eq!(info.name, "transformer"); // Should use default
  assert!(info.type_name.contains("TestCloneTransformer"));
}

// Test create_error_context uses component_info correctly
#[test]
fn test_transformer_create_error_context_uses_component_info() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("test_transformer".to_string()),
  };
  let context = Transformer::create_error_context(&test_transformer, Some(42));

  let info = Transformer::component_info(&test_transformer);
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_type, info.type_name);
}

// Test handle_error calls config() internally
#[test]
fn test_transformer_handle_error_internal_call() {
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };

  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 0,
  };

  // handle_error internally calls config().error_strategy()
  // This tests line 328 in transformer.rs
  let action = Transformer::handle_error(&test_transformer, &error);
  assert!(matches!(action, ErrorAction::Skip));
}

// Test set_config calls set_config_impl internally
#[tokio::test]
async fn test_transformer_set_config_internal_call() {
  // set_config internally calls set_config_impl
  // This tests line 272 in transformer.rs
  let mut test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let config = TransformerConfig::default().with_name("internal_test".to_string());

  Transformer::set_config(&mut test_transformer, config);
  assert_eq!(
    test_transformer.config().name(),
    Some("internal_test".to_string())
  );
}

// Test config() calls get_config_impl internally
#[test]
fn test_transformer_config_internal_call() {
  // config() internally calls get_config_impl()
  // This tests line 281 in transformer.rs
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("config_test".to_string()),
  };
  let config_ref = Transformer::config(&test_transformer);
  assert_eq!(config_ref.name(), Some("config_test".to_string()));
}

// Test config_mut() calls get_config_mut_impl internally
#[test]
fn test_transformer_config_mut_internal_call() {
  // config_mut() internally calls get_config_mut_impl()
  // This tests line 290 in transformer.rs
  let mut test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let config_mut_ref = Transformer::config_mut(&mut test_transformer);
  config_mut_ref.name = Some("mut_test".to_string());
  assert_eq!(
    test_transformer.config().name(),
    Some("mut_test".to_string())
  );
}

// Test with_name calls get_config_impl and set_config internally
#[test]
fn test_transformer_with_name_internal_call() {
  // with_name internally calls get_config_impl().clone() and set_config
  // This tests lines 307-311 in transformer.rs
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default()
      .with_name("original".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  };

  // This should clone the config, preserve error_strategy, and set name
  let named = Transformer::with_name(test_transformer, "internal_name".to_string());
  assert_eq!(named.config().name(), Some("internal_name".to_string()));
  // Error strategy should be preserved (was Skip)
  assert!(matches!(
    named.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test all branches in handle_error default implementation
#[test]
fn test_transformer_handle_error_all_branches() {
  let base_context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let base_component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };

  // Branch 1: ErrorStrategy::Stop (line 329)
  let transformer1 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop),
  };
  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Transformer::handle_error(&transformer1, &error1),
    ErrorAction::Stop
  ));

  // Branch 2: ErrorStrategy::Skip (line 330)
  let transformer2 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };
  assert!(matches!(
    Transformer::handle_error(&transformer2, &error1),
    ErrorAction::Skip
  ));

  // Branch 3: ErrorStrategy::Retry(n) if error.retries < n (line 331)
  let transformer3 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(10)),
  };
  let error2 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 5, // Less than 10
  };
  assert!(matches!(
    Transformer::handle_error(&transformer3, &error2),
    ErrorAction::Retry
  ));

  // Branch 4: ErrorStrategy::Custom(ref handler) (line 332)
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Skip);
  let transformer4 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(custom_handler),
  };
  assert!(matches!(
    Transformer::handle_error(&transformer4, &error1),
    ErrorAction::Skip
  ));

  // Branch 5: _ => ErrorAction::Stop (line 333) - Retry exceeded
  let transformer5 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)),
  };
  let error3 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context,
    component: base_component,
    retries: 10, // Exceeds 3
  };
  assert!(matches!(
    Transformer::handle_error(&transformer5, &error3),
    ErrorAction::Stop
  ));
}

// Test component_info calls config() internally
#[test]
fn test_transformer_component_info_internal_call() {
  // component_info internally calls config().name().unwrap_or_else(...)
  // This tests lines 373-376 in transformer.rs
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("component_info_test".to_string()),
  };

  // Call component_info which should execute both lines
  let info1 = Transformer::component_info(&test_transformer);
  assert_eq!(info1.name, "component_info_test");
  assert!(info1.type_name.contains("TestCloneTransformer"));

  // Call again to ensure lines are executed
  let info2 = Transformer::component_info(&test_transformer);
  assert_eq!(info2.name, "component_info_test");
  assert_eq!(info1.type_name, info2.type_name);

  // Test with None name - should use unwrap_or_else default
  let test_transformer_no_name = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };
  let info3 = Transformer::component_info(&test_transformer_no_name);
  assert_eq!(info3.name, "transformer"); // Trait default uses "transformer"
}

// Test create_error_context executes all lines
#[test]
fn test_transformer_create_error_context_all_lines() {
  // Test lines 354-359 in transformer.rs
  // Line 355: timestamp: chrono::Utc::now()
  // Line 356: item
  // Line 357: component_name: self.component_info().name
  // Line 358: component_type: self.component_info().type_name
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("context_all_lines".to_string()),
  };

  // Create context with Some - tests line 356 with Some
  let before = chrono::Utc::now();
  let context1 = Transformer::create_error_context(&test_transformer, Some(42));
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 355)
  assert!(context1.timestamp >= before);
  assert!(context1.timestamp <= after);

  // Verify item is set (line 356)
  assert_eq!(context1.item, Some(42));

  // Verify component_name uses component_info (line 357)
  let info = Transformer::component_info(&test_transformer);
  assert_eq!(context1.component_name, info.name);

  // Verify component_type uses component_info (line 358)
  assert_eq!(context1.component_type, info.type_name);

  // Create context with None - tests line 356 with None
  let context2 = Transformer::create_error_context(&test_transformer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, info.name);
}

// Test handle_error uses config() internally (line 328)
#[test]
fn test_transformer_handle_error_uses_config() {
  let test_transformer1 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop),
  };

  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 0,
  };

  // This should call config().error_strategy() internally (line 328)
  let action1 = Transformer::handle_error(&test_transformer1, &error);
  assert!(matches!(action1, ErrorAction::Stop));

  // Change strategy and verify config() is called again
  let test_transformer2 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };
  let action2 = Transformer::handle_error(&test_transformer2, &error);
  assert!(matches!(action2, ErrorAction::Skip));
}

// Test TransformerConfig Default implementation comprehensive
#[test]
fn test_transformer_config_default_implementation_comprehensive() {
  // Test lines 47-52 in transformer.rs: Default implementation
  let config: TransformerConfig<i32> = Default::default();

  // Verify line 49 is executed
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // Verify line 50 is executed
  assert_eq!(config.name, None);

  // Test that Default is actually called for different types
  let config2: TransformerConfig<String> = Default::default();
  assert!(matches!(config2.error_strategy, ErrorStrategy::Stop));
  assert_eq!(config2.name, None);
}

// Test TransformerConfig with_error_strategy mutation line
#[test]
fn test_transformer_config_with_error_strategy_mutation_line() {
  // Test line 62 in transformer.rs: self.error_strategy = strategy;
  let config = TransformerConfig::<i32>::default();

  // Before mutation
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // After with_error_strategy (line 62)
  let config2 = config.with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config2.error_strategy, ErrorStrategy::Skip));
}

// Test TransformerConfig with_name mutation line
#[test]
fn test_transformer_config_with_name_mutation_line() {
  // Test line 72 in transformer.rs: self.name = Some(name);
  let config = TransformerConfig::<i32>::default();

  // Before mutation
  assert_eq!(config.name, None);

  // After with_name (line 72)
  let config2 = config.with_name("mutation_test".to_string());
  assert_eq!(config2.name, Some("mutation_test".to_string()));
}

// Test that with_config clone line is executed
#[test]
fn test_transformer_with_config_clone_line() {
  // Test line 261 in transformer.rs: let mut this = self.clone();
  let transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("clone_test".to_string()),
  };

  // Clone should be called here (line 261)
  let transformer2 = transformer.clone();
  assert_eq!(transformer2.config().name(), Some("clone_test".to_string()));

  // Verify they are different instances
  assert_eq!(transformer.config().name(), transformer2.config().name());

  // Now test with_config which also calls clone (line 261)
  let config = TransformerConfig::default().with_name("after_config".to_string());
  let configured = Transformer::with_config(&transformer, config);
  assert_eq!(configured.config().name(), Some("after_config".to_string()));
}

// Test TransformerConfig error_strategy() clone
#[test]
fn test_transformer_config_error_strategy_clone_all() {
  // Test line 78 in transformer.rs: self.error_strategy.clone()
  let config = TransformerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(99));

  // This calls error_strategy() which clones the strategy (line 78)
  let strategy = config.error_strategy();
  match strategy {
    ErrorStrategy::Retry(99) => {}
    _ => panic!("Expected Retry(99)"),
  }

  // Call again to verify it's a clone operation
  let strategy2 = config.error_strategy();
  match strategy2 {
    ErrorStrategy::Retry(99) => {}
    _ => panic!("Expected Retry(99)"),
  }
}

// Test TransformerConfig name() clone
#[test]
fn test_transformer_config_name_clone_all() {
  // Test line 83 in transformer.rs: self.name.clone()
  let config = TransformerConfig::<i32>::default().with_name("name_clone_test".to_string());

  // This calls name() which clones the Option<String> (line 83)
  let name = config.name();
  assert_eq!(name, Some("name_clone_test".to_string()));

  // Call again to verify it's a clone operation
  let name2 = config.name();
  assert_eq!(name2, Some("name_clone_test".to_string()));

  // Test with None
  let config_none = TransformerConfig::<i32>::default();
  assert_eq!(config_none.name(), None);
}

// Test TransformerPorts blanket implementation type usage
#[test]
fn test_transformer_ports_blanket_impl_type_usage() {
  use crate::port::PortList;
  use crate::transformer::TransformerPorts;

  // Test that lines 27-28 in transformer.rs are executed
  // type DefaultInputPorts = (T::Input,);
  // type DefaultOutputPorts = (T::Output,);

  // Create a function that requires the types to be resolved
  fn use_default_ports<T>() -> (
    std::marker::PhantomData<<T as TransformerPorts>::DefaultInputPorts>,
    std::marker::PhantomData<<T as TransformerPorts>::DefaultOutputPorts>,
  )
  where
    T: Transformer + TransformerPorts,
    T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <T as TransformerPorts>::DefaultInputPorts: PortList,
    <T as TransformerPorts>::DefaultOutputPorts: PortList,
  {
    (std::marker::PhantomData, std::marker::PhantomData)
  }

  // This forces the types to be resolved, executing the blanket impl
  let _phantom = use_default_ports::<MapTransformer<fn(i32) -> i32, i32, i32>>();

  // Verify the types are correct by using them
  type TestInputPorts =
    <MapTransformer<fn(i32) -> i32, i32, i32> as TransformerPorts>::DefaultInputPorts;
  type TestOutputPorts =
    <MapTransformer<fn(i32) -> i32, i32, i32> as TransformerPorts>::DefaultOutputPorts;
  fn check_size<T>() -> usize {
    std::mem::size_of::<T>()
  }
  let input_size = check_size::<TestInputPorts>();
  let output_size = check_size::<TestOutputPorts>();
  let expected_size = std::mem::size_of::<(i32,)>();
  assert_eq!(input_size, expected_size);
  assert_eq!(output_size, expected_size);
}

// Test component_info trait default - ensure it calls config().name().unwrap_or_else()
#[test]
fn test_transformer_component_info_trait_default_calls_config() {
  // Test lines 373-378 in transformer.rs: component_info default implementation
  // Line 373-376: name: self.config().name().unwrap_or_else(|| "transformer".to_string())
  // Line 377: type_name: std::any::type_name::<Self>().to_string()

  // Test with Some(name)
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("config_test".to_string()),
  };
  let info = Transformer::component_info(&test_transformer);
  assert_eq!(info.name, "config_test");
  assert!(info.type_name.contains("TestCloneTransformer"));

  // Test with None - should use unwrap_or_else default
  let test_transformer2 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(), // name is None
  };
  let info2 = Transformer::component_info(&test_transformer2);
  assert_eq!(info2.name, "transformer"); // Trait default uses "transformer"

  // Verify it calls config().name() and unwrap_or_else
  // This tests the full path: config() -> name() -> unwrap_or_else()
  assert_eq!(
    info2.name,
    test_transformer2
      .config()
      .name()
      .unwrap_or_else(|| "transformer".to_string())
  );
}

// Test with_name trait default - ensure it clones config and preserves error_strategy
#[test]
fn test_transformer_with_name_trait_default_clone_and_preserve() {
  // Test lines 303-312 in transformer.rs: with_name default implementation
  // Line 307: let config = self.get_config_impl().clone();
  // Line 308-311: self.set_config(TransformerConfig { error_strategy: config.error_strategy, name: Some(name) })

  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default()
      .with_name("original_name".to_string())
      .with_error_strategy(ErrorStrategy::Retry(7)),
  };

  // This should clone the config, preserve error_strategy, and set name
  let named = Transformer::with_name(test_transformer, "new_name".to_string());

  // Verify name is updated (line 310)
  assert_eq!(named.config().name(), Some("new_name".to_string()));

  // Verify error_strategy is preserved (line 309)
  match named.config().error_strategy() {
    ErrorStrategy::Retry(7) => {}
    _ => panic!("Expected Retry(7) strategy to be preserved"),
  }
}

// Test create_error_context trait default implementation
#[test]
fn test_transformer_create_error_context_trait_default() {
  // Test that create_error_context trait default is called correctly
  // MapTransformer overrides this, so use TestCloneTransformer
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("trait_default_test".to_string()),
  };

  // This should use the trait default implementation
  // Lines 354-359 in transformer.rs
  let before = chrono::Utc::now();
  let context = Transformer::create_error_context(&test_transformer, Some(42));
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 355)
  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);

  // Verify item is set (line 356)
  assert_eq!(context.item, Some(42));

  // Verify component_name uses component_info (line 357)
  let info = Transformer::component_info(&test_transformer);
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_name, "trait_default_test");

  // Verify component_type uses component_info (line 358)
  assert_eq!(context.component_type, info.type_name);

  // Test with None
  let context2 = Transformer::create_error_context(&test_transformer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, info.name);
}

// Test handle_error trait default - ensure it calls config().error_strategy()
#[test]
fn test_transformer_handle_error_trait_default_calls_config() {
  // Test that line 328 in transformer.rs: match self.config().error_strategy() is executed
  // Use TestCloneTransformer which uses trait default
  let test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Stop),
  };

  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 0,
  };

  // This should call config().error_strategy() internally (line 328)
  let action = Transformer::handle_error(&test_transformer, &error);
  assert!(matches!(action, ErrorAction::Stop));

  // Test with different strategy to ensure config() is called again
  let test_transformer2 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };
  let action2 = Transformer::handle_error(&test_transformer2, &error);
  assert!(matches!(action2, ErrorAction::Skip));
}

// Test TransformerConfig struct field access
#[test]
fn test_transformer_config_field_access() {
  // Test that struct fields are accessible (lines 41, 43)
  let config = TransformerConfig::<i32> {
    error_strategy: ErrorStrategy::Skip,
    name: Some("field_test".to_string()),
  };

  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
  assert_eq!(config.name, Some("field_test".to_string()));

  // Test accessing fields directly
  let config2 = TransformerConfig::<i32> {
    error_strategy: ErrorStrategy::Retry(7),
    name: Some("direct_access".to_string()),
  };

  assert!(matches!(config2.error_strategy(), ErrorStrategy::Retry(7)));
  assert_eq!(config2.name(), Some("direct_access".to_string()));
}

// Test that config_mut allows direct field mutation
#[test]
fn test_transformer_config_mut_direct_mutation() {
  let mut test_transformer = TestCloneTransformer::<i32> {
    config: TransformerConfig::default(),
  };

  // Test direct field access through config_mut
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Retry);
  test_transformer.config_mut().error_strategy = custom_handler;
  test_transformer.config_mut().name = Some("direct_mut".to_string());

  assert_eq!(
    test_transformer.config().name(),
    Some("direct_mut".to_string())
  );
  // Can't easily test Custom equality, but we can test it exists
  match test_transformer.config().error_strategy() {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom strategy"),
  }
}

// Test that with_config actually uses clone correctly
#[test]
fn test_transformer_with_config_clone_behavior() {
  let transformer1 = TestCloneTransformer::<i32> {
    config: TransformerConfig::default().with_name("original".to_string()),
  };

  // Clone the transformer
  let transformer2 = transformer1.clone();
  assert_eq!(transformer2.config().name(), Some("original".to_string()));

  // Test that with_config creates a new instance and calls set_config
  let new_config = TransformerConfig::default().with_name("new".to_string());
  let configured = transformer2.with_config(new_config);
  assert_eq!(configured.config().name(), Some("new".to_string()));

  // Verify it's a different instance
  assert_ne!(transformer2.config().name(), configured.config().name());
}

// Test TransformerPorts blanket implementation for both InputPorts and OutputPorts
#[test]
fn test_transformer_ports_blanket_impl_both_ports() {
  use crate::port::PortList;
  use crate::transformer::TransformerPorts;

  // Test that both DefaultInputPorts and DefaultOutputPorts are set correctly
  fn check_ports_types<T>()
  where
    T: Transformer + TransformerPorts,
    T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <T as TransformerPorts>::DefaultInputPorts: PortList,
    <T as TransformerPorts>::DefaultOutputPorts: PortList,
  {
    // Just verifying it compiles
  }

  check_ports_types::<MapTransformer<fn(i32) -> i32, i32, i32>>();
  check_ports_types::<MapTransformer<fn(String) -> String, String, String>>();

  // Verify both types are correct
  type MapInputPorts =
    <MapTransformer<fn(i32) -> i32, i32, i32> as TransformerPorts>::DefaultInputPorts;
  type MapOutputPorts =
    <MapTransformer<fn(i32) -> i32, i32, i32> as TransformerPorts>::DefaultOutputPorts;

  fn assert_type<T: PortList>() {}
  assert_type::<MapInputPorts>();
  assert_type::<MapOutputPorts>();
  assert_type::<(i32,)>();
}
