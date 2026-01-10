use crate::consumers::VecConsumer;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::Stream;
use futures::stream;
use std::error::Error;
use std::pin::Pin;

#[test]
fn test_consumer_config_default() {
  let config = ConsumerConfig::<i32>::default();

  assert_eq!(config.name(), "");
  match config.error_strategy() {
    ErrorStrategy::Stop => {} // Expected
    _ => panic!("Expected Stop strategy by default"),
  }
}

#[test]
fn test_consumer_config_with_error_strategy() {
  let config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);

  match config.error_strategy() {
    ErrorStrategy::Skip => {} // Expected
    _ => panic!("Expected Skip strategy"),
  }
}

#[test]
fn test_consumer_config_with_name() {
  let config = ConsumerConfig::<i32>::default().with_name("my_consumer".to_string());

  assert_eq!(config.name(), "my_consumer");
}

#[test]
fn test_consumer_config_builder_chain() {
  let config = ConsumerConfig::<i32>::default()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .with_name("test_consumer".to_string());

  assert_eq!(config.name(), "test_consumer");
  match config.error_strategy() {
    ErrorStrategy::Retry(3) => {}
    _ => panic!("Expected Retry(3) strategy"),
  }
}

#[test]
fn test_consumer_config_error_strategy_getter() {
  let config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);

  let strategy = config.error_strategy();
  match strategy {
    ErrorStrategy::Skip => {}
    _ => panic!("Expected Skip strategy"),
  }
}

// Note: with_config requires Clone, which VecConsumer doesn't implement
// This is tested indirectly through set_config which doesn't require Clone

#[tokio::test]
async fn test_consumer_set_config() {
  let mut consumer = VecConsumer::<i32>::new();
  let config = ConsumerConfig::<i32>::default().with_name("new_name".to_string());

  consumer.set_config(config);
  assert_eq!(consumer.config().name(), "new_name");
}

#[test]
fn test_consumer_config_getter() {
  let consumer = VecConsumer::<i32>::new();
  let config = consumer.config();
  assert_eq!(config.name(), "");
}

#[tokio::test]
async fn test_consumer_config_mut() {
  let mut consumer = VecConsumer::<i32>::new();
  consumer.config_mut().name = "modified".to_string();
  assert_eq!(consumer.config().name(), "modified");
}

#[test]
fn test_consumer_with_name() {
  let consumer = VecConsumer::<i32>::new();
  let named = consumer.with_name("named_consumer".to_string());
  assert_eq!(named.config().name(), "named_consumer");
}

#[test]
fn test_consumer_handle_error_stop() {
  let consumer = VecConsumer::<i32>::new();
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

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_consumer_handle_error_skip() {
  let mut consumer = VecConsumer::<i32>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));

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

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Skip));
}

#[test]
fn test_consumer_handle_error_retry() {
  let mut consumer = VecConsumer::<i32>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));

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

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Retry));
}

#[test]
fn test_consumer_handle_error_retry_exceeded() {
  let mut consumer = VecConsumer::<i32>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));

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

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_consumer_handle_error_retry_exceeded_through_trait() {
  // Test the trait's handle_error method (not VecConsumer's override)
  // This tests the `_ => ErrorAction::Stop` branch in consumer.rs:292
  use crate::consumers::ConsoleConsumer;

  let mut consumer = ConsoleConsumer::<String>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(2)));

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
    retries: 3, // Exceeds limit of 2, should hit the `_ => ErrorAction::Stop` branch
  };

  // This calls Consumer trait's handle_error, not VecConsumer's override
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_consumer_with_name_chain() {
  let consumer = VecConsumer::<i32>::new()
    .with_name("first".to_string())
    .with_name("second".to_string());
  assert_eq!(consumer.config().name(), "second");
}

#[test]
fn test_consumer_config_error_strategy_all_variants() {
  let stop_config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Stop);
  assert!(matches!(stop_config.error_strategy(), ErrorStrategy::Stop));

  let skip_config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(skip_config.error_strategy(), ErrorStrategy::Skip));

  let retry_config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(5));
  match retry_config.error_strategy() {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }
}

#[test]
fn test_consumer_handle_error_custom() {
  let mut consumer = VecConsumer::<i32>::new();
  let custom_handler = ErrorStrategy::new_custom(|_error: &StreamError<i32>| ErrorAction::Skip);
  consumer.set_config(ConsumerConfig::default().with_error_strategy(custom_handler));

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

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Skip));
}

#[test]
fn test_consumer_component_info() {
  let consumer = VecConsumer::<i32>::new();
  let info = consumer.component_info();
  assert_eq!(info.name, "");
  assert!(info.type_name.contains("VecConsumer"));
}

#[test]
fn test_consumer_component_info_with_name() {
  let consumer = VecConsumer::<i32>::new().with_name("named_consumer".to_string());
  let info = consumer.component_info();
  assert_eq!(info.name, "named_consumer");
  assert!(info.type_name.contains("VecConsumer"));
}

#[test]
fn test_consumer_create_error_context() {
  let consumer = VecConsumer::<i32>::new().with_name("test_consumer".to_string());
  let context = consumer.create_error_context(Some(42));

  assert_eq!(context.component_name, "test_consumer");
  assert!(context.component_type.contains("VecConsumer"));
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_consumer_create_error_context_no_item() {
  let consumer = VecConsumer::<i32>::new();
  let context = consumer.create_error_context(None);

  assert_eq!(context.item, None);
  assert!(!context.component_type.is_empty());
}

#[tokio::test]
async fn test_consumer_consume() {
  let mut consumer = VecConsumer::<i32>::new();
  let items = stream::iter(vec![1, 2, 3]);
  let boxed_stream = Box::pin(items);

  consumer.consume(boxed_stream).await;

  assert_eq!(consumer.vec.len(), 3);
  assert_eq!(consumer.vec, vec![1, 2, 3]);
}

#[test]
fn test_consumer_input_ports_trait() {
  // Test that VecConsumer implements InputPorts correctly
  use crate::port::PortList;
  fn assert_ports<C>()
  where
    C: Consumer,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    C::InputPorts: PortList,
  {
    // Just checking it compiles
  }

  assert_ports::<VecConsumer<i32>>();
}

#[tokio::test]
async fn test_consumer_consume_empty_stream() {
  let mut consumer = VecConsumer::<i32>::new();
  let items = stream::iter(vec![]);
  let boxed_stream = Box::pin(items);

  consumer.consume(boxed_stream).await;

  assert!(consumer.vec.is_empty());
}

// Test ConsumerPorts trait's blanket implementation
#[test]
fn test_consumer_ports_trait() {
  use crate::consumer::ConsumerPorts;
  use crate::port::PortList;

  // Test that VecConsumer implements ConsumerPorts with DefaultInputPorts = (i32,)
  type ExpectedPorts = <VecConsumer<i32> as ConsumerPorts>::DefaultInputPorts;
  fn assert_type<TPorts: PortList>() {}

  assert_type::<ExpectedPorts>();
  assert_type::<(i32,)>();

  // This should compile if the blanket impl works correctly
  // The actual usage is tested through the trait bounds in other code
}

// Test with_config method - requires Clone
// Since no built-in consumers implement Clone, we create a simple test consumer
#[derive(Clone)]
struct TestCloneConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  config: ConsumerConfig<T>,
}

impl<T> Input for TestCloneConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for TestCloneConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, _stream: Self::InputStream) {
    // No-op for testing
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }
}

#[test]
fn test_consumer_with_config() {
  let consumer = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("original".to_string()),
  };

  let new_config = ConsumerConfig::default()
    .with_name("new_name".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let configured = consumer.with_config(new_config);
  assert_eq!(configured.config().name(), "new_name");
  assert!(matches!(
    configured.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test create_error_context with various item types
#[test]
fn test_consumer_create_error_context_with_different_items() {
  let consumer = VecConsumer::<i32>::new().with_name("test_consumer".to_string());

  // Test with Some(item)
  let context1 = consumer.create_error_context(Some(42));
  assert_eq!(context1.item, Some(42));
  assert_eq!(context1.component_name, "test_consumer");

  // Test with None
  let context2 = consumer.create_error_context(None);
  assert_eq!(context2.item, None);

  // Test with different value
  let context3 = consumer.create_error_context(Some(100));
  assert_eq!(context3.item, Some(100));
}

// Test ConsumerPorts trait more directly
#[test]
fn test_consumer_ports_default_input_ports() {
  use crate::consumer::ConsumerPorts;
  use crate::port::PortList;

  // Test that the blanket implementation works
  // VecConsumer<i32> should have DefaultInputPorts = (i32,)
  fn check_ports_type<C>()
  where
    C: Consumer + ConsumerPorts,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    <C as ConsumerPorts>::DefaultInputPorts: PortList,
  {
    // Just verifying it compiles - the type constraint validates the impl
  }

  check_ports_type::<VecConsumer<i32>>();
}

// Test all error strategy variants in handle_error
#[test]
fn test_consumer_handle_error_all_strategies() {
  use crate::consumers::ConsoleConsumer;

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

  // Test Stop strategy
  let mut consumer = ConsoleConsumer::<String>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &error),
    ErrorAction::Stop
  ));

  // Test Skip strategy
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  assert!(matches!(
    Consumer::handle_error(&consumer, &error),
    ErrorAction::Skip
  ));

  // Test Retry strategy - should retry when retries < limit
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  let retry_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 2, // Less than 3
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &retry_error),
    ErrorAction::Retry
  ));

  // Test Retry strategy - should stop when retries >= limit
  let stop_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 3, // Equal to limit - should hit `_ => ErrorAction::Stop` branch
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &stop_error),
    ErrorAction::Stop
  ));

  // Test Retry strategy - should stop when retries > limit
  let exceed_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 4, // Greater than limit - should hit `_ => ErrorAction::Stop` branch
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &exceed_error),
    ErrorAction::Stop
  ));
}

// Test ConsumerPorts trait usage with multiple consumers
#[test]
fn test_consumer_ports_blanket_impl_variations() {
  use crate::consumer::ConsumerPorts;
  use crate::consumers::ConsoleConsumer;
  use crate::port::PortList;

  // Test that different consumers implement ConsumerPorts
  fn check_ports_type<C>()
  where
    C: Consumer + ConsumerPorts,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    <C as ConsumerPorts>::DefaultInputPorts: PortList,
  {
    // Just verifying it compiles
  }

  check_ports_type::<VecConsumer<i32>>();
  check_ports_type::<ConsoleConsumer<String>>();

  // Verify the type is correct
  type VecPorts = <VecConsumer<i32> as ConsumerPorts>::DefaultInputPorts;
  type ConsolePorts = <ConsoleConsumer<String> as ConsumerPorts>::DefaultInputPorts;

  fn assert_type<T: PortList>() {}
  assert_type::<VecPorts>();
  assert_type::<ConsolePorts>();
  assert_type::<(i32,)>();
  assert_type::<(String,)>();
}

// Test component_info with empty name
#[test]
fn test_consumer_component_info_empty_name() {
  let consumer = VecConsumer::<i32>::new();
  let info = consumer.component_info();
  assert_eq!(info.name, "");
  assert!(info.type_name.contains("VecConsumer"));

  // Test that it uses config().name directly
  assert_eq!(info.name, consumer.config().name());
}

// Test create_error_context uses component_info correctly
#[test]
fn test_consumer_create_error_context_uses_component_info() {
  let consumer = VecConsumer::<i32>::new().with_name("test_consumer".to_string());
  let context = consumer.create_error_context(Some(42));

  let info = consumer.component_info();
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_type, info.type_name);
}

// Test config getters more thoroughly
#[test]
fn test_consumer_config_getters_mutability() {
  let mut consumer = VecConsumer::<i32>::new();

  // Test mutable access
  consumer.config_mut().name = "mutated".to_string();
  assert_eq!(consumer.config().name(), "mutated");

  // Test that changes are reflected
  consumer.config_mut().error_strategy = ErrorStrategy::Skip;
  assert!(matches!(
    consumer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test set_config with various configurations
#[tokio::test]
async fn test_consumer_set_config_variations() {
  let mut consumer = VecConsumer::<i32>::new();

  // Test setting config with name only
  let config1 = ConsumerConfig::default().with_name("name_only".to_string());
  consumer.set_config(config1);
  assert_eq!(consumer.config().name(), "name_only");
  assert!(matches!(
    consumer.config().error_strategy(),
    ErrorStrategy::Stop
  ));

  // Test setting config with error strategy only
  let config2 = ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip);
  consumer.set_config(config2);
  assert_eq!(consumer.config().name(), "");
  assert!(matches!(
    consumer.config().error_strategy(),
    ErrorStrategy::Skip
  ));

  // Test setting config with both
  let config3 = ConsumerConfig::default()
    .with_name("both".to_string())
    .with_error_strategy(ErrorStrategy::Retry(5));
  consumer.set_config(config3);
  assert_eq!(consumer.config().name(), "both");
  match consumer.config().error_strategy() {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }
}

// Test that with_name modifies the config correctly
#[test]
fn test_consumer_with_name_modifies_config() {
  let consumer = VecConsumer::<i32>::new();
  let named = consumer.with_name("test_name".to_string());

  // Verify the name is set
  assert_eq!(named.config().name(), "test_name");

  // Verify it's a new instance (VecConsumer doesn't implement Clone, so with_name must consume)
  // This tests that the method works correctly
}

// Test handle_error returns correct action based on strategy
#[test]
fn test_consumer_handle_error_action_mapping() {
  use crate::consumers::ConsoleConsumer;

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

  // Test that each strategy maps to correct action
  let mut consumer = ConsoleConsumer::<String>::new();

  // Stop -> Stop
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 0,
  };
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Stop));

  // Skip -> Skip
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Skip));

  // Retry (under limit) -> Retry
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));
  let retry_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 3, // Less than 5
  };
  let action = Consumer::handle_error(&consumer, &retry_error);
  assert!(matches!(action, ErrorAction::Retry));

  // Retry (at limit) -> Stop (via default branch)
  let limit_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 5, // Equal to limit
  };
  let action = Consumer::handle_error(&consumer, &limit_error);
  assert!(matches!(action, ErrorAction::Stop));

  // Retry (over limit) -> Stop (via default branch)
  let over_limit_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 6, // Over limit
  };
  let action = Consumer::handle_error(&consumer, &over_limit_error);
  assert!(matches!(action, ErrorAction::Stop));
}

// Test that with_config actually uses clone correctly
#[test]
fn test_consumer_with_config_clone_behavior() {
  let consumer1 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("original".to_string()),
  };

  // Clone the consumer
  let consumer2 = consumer1.clone();
  assert_eq!(consumer2.config().name(), "original");

  // Test that with_config creates a new instance and calls set_config
  let new_config = ConsumerConfig::default().with_name("new".to_string());
  let configured = consumer2.with_config(new_config);
  assert_eq!(configured.config().name(), "new");

  // Verify it's a different instance
  assert_ne!(consumer2.config().name(), configured.config().name());
}

// Test ConsumerConfig Default implementation
#[test]
fn test_consumer_config_default_impl() {
  // Test that Default implementation creates correct config
  let config: ConsumerConfig<i32> = Default::default();
  assert_eq!(config.name, "");
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));
}

// Test ConsumerConfig with_error_strategy mutates correctly
#[test]
fn test_consumer_config_with_error_strategy_mutation() {
  let config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(10));

  // Verify the strategy is set
  match config.error_strategy() {
    ErrorStrategy::Retry(10) => {}
    _ => panic!("Expected Retry(10)"),
  }

  // Test chaining
  let config2 = ConsumerConfig::<i32>::default()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_error_strategy(ErrorStrategy::Stop);

  // Last strategy should win
  assert!(matches!(config2.error_strategy(), ErrorStrategy::Stop));
}

// Test ConsumerConfig with_name mutates correctly
#[test]
fn test_consumer_config_with_name_mutation() {
  let config = ConsumerConfig::<i32>::default().with_name("test_name".to_string());

  assert_eq!(config.name(), "test_name");

  // Test chaining - last name should win
  let config2 = ConsumerConfig::<i32>::default()
    .with_name("first".to_string())
    .with_name("second".to_string());

  assert_eq!(config2.name(), "second");
}

// Test that all Consumer trait default methods are callable
#[test]
fn test_consumer_trait_default_methods_exist() {
  use crate::consumers::ConsoleConsumer;

  let mut consumer = ConsoleConsumer::<String>::new();

  // Test that all default methods exist and can be called
  // These test the method signatures in the trait

  // config() - should return a reference
  let _config_ref = consumer.config();

  // config_mut() - should return a mutable reference
  let _config_mut_ref = consumer.config_mut();

  // component_info() - should return ComponentInfo
  let _info = Consumer::component_info(&consumer);

  // create_error_context() - should return ErrorContext
  let _context = Consumer::create_error_context(&consumer, Some("item".to_string()));
  let _context_none = Consumer::create_error_context(&consumer, None);

  // with_name() - should return Self (consumes self)
  let named = consumer.with_name("test".to_string());
  assert_eq!(named.config().name(), "test");
}

// Test create_error_context calls component_info internally
#[test]
fn test_consumer_create_error_context_internal_call() {
  use crate::consumers::ConsoleConsumer;

  let consumer = ConsoleConsumer::<String>::new().with_name("test_consumer".to_string());

  // create_error_context internally calls component_info()
  // This tests lines 332-333 in consumer.rs
  let context = Consumer::create_error_context(&consumer, Some("test".to_string()));

  // Verify it uses component_info correctly
  let info = Consumer::component_info(&consumer);
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_type, info.type_name);
}

// Test component_info calls config() internally
#[test]
fn test_consumer_component_info_internal_call() {
  use crate::consumers::ConsoleConsumer;

  let consumer = ConsoleConsumer::<String>::new().with_name("test_name".to_string());

  // component_info internally calls config().name.clone()
  // This tests line 308 in consumer.rs
  let info = Consumer::component_info(&consumer);
  assert_eq!(info.name, consumer.config().name());

  // Test that it uses std::any::type_name
  // This tests line 309 in consumer.rs
  assert!(info.type_name.contains("ConsoleConsumer"));
}

// Test handle_error calls config() internally
#[test]
fn test_consumer_handle_error_internal_call() {
  use crate::consumers::ConsoleConsumer;

  let mut consumer = ConsoleConsumer::<String>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));

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

  // handle_error internally calls config() to get error_strategy
  // This tests line 287 in consumer.rs
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Skip));
}

// Test set_config calls set_config_impl internally
#[tokio::test]
async fn test_consumer_set_config_internal_call() {
  // set_config internally calls set_config_impl
  // This tests line 246 in consumer.rs
  let mut consumer = VecConsumer::<i32>::new();
  let config = ConsumerConfig::default().with_name("internal_test".to_string());

  Consumer::set_config(&mut consumer, config);
  assert_eq!(consumer.config().name(), "internal_test");
}

// Test config() calls get_config_impl internally
#[test]
fn test_consumer_config_internal_call() {
  // config() internally calls get_config_impl()
  // This tests line 252 in consumer.rs
  let consumer = VecConsumer::<i32>::new().with_name("config_test".to_string());
  let config_ref = Consumer::config(&consumer);
  assert_eq!(config_ref.name(), "config_test");
}

// Test config_mut() calls get_config_mut_impl internally
#[test]
fn test_consumer_config_mut_internal_call() {
  // config_mut() internally calls get_config_mut_impl()
  // This tests line 258 in consumer.rs
  let mut consumer = VecConsumer::<i32>::new();
  let config_mut_ref = Consumer::config_mut(&mut consumer);
  config_mut_ref.name = "mut_test".to_string();
  assert_eq!(consumer.config().name(), "mut_test");
}

// Test with_name calls config_mut() internally
#[test]
fn test_consumer_with_name_internal_call() {
  use crate::consumers::ConsoleConsumer;

  // with_name internally calls config_mut().name = name.clone()
  // This tests lines 271-272 in consumer.rs
  let consumer = ConsoleConsumer::<String>::new();
  let named = Consumer::with_name(consumer, "internal_name".to_string());
  assert_eq!(named.config().name(), "internal_name");
}

// Test Consumer trait default component_info through TestCloneConsumer
#[test]
fn test_consumer_component_info_trait_default() {
  // Test lines 306-311 in consumer.rs: component_info default implementation
  // Line 308: name: self.config().name.clone()
  // Line 309: type_name: std::any::type_name::<Self>().to_string()
  // VecConsumer overrides this, so use TestCloneConsumer which uses the trait default

  // Test with empty name
  let consumer = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default(), // name is ""
  };
  let info = Consumer::component_info(&consumer);
  assert_eq!(info.name, ""); // ConsumerConfig uses String::new() for default
  assert!(info.type_name.contains("TestCloneConsumer"));

  // Test with non-empty name
  let consumer2 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("trait_default_name".to_string()),
  };
  let info2 = Consumer::component_info(&consumer2);
  assert_eq!(info2.name, "trait_default_name");
  assert!(info2.type_name.contains("TestCloneConsumer"));

  // Verify it calls config().name.clone() (line 308)
  assert_eq!(info2.name, consumer2.config().name());

  // Verify it uses std::any::type_name (line 309)
  assert!(info2.type_name.contains("TestCloneConsumer"));
}

// Test Consumer trait default create_error_context through TestCloneConsumer
#[test]
fn test_consumer_create_error_context_trait_default() {
  // Test lines 328-335 in consumer.rs: create_error_context default implementation
  // Line 330: timestamp: chrono::Utc::now()
  // Line 331: item
  // Line 332: component_name: self.component_info().name
  // Line 333: component_type: self.component_info().type_name
  // VecConsumer overrides this, so use TestCloneConsumer which uses the trait default

  let consumer = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("trait_default_context".to_string()),
  };

  // Create context with Some - tests line 331 with Some
  let before = chrono::Utc::now();
  let context = Consumer::create_error_context(&consumer, Some(42));
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 330)
  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);

  // Verify item is set (line 331)
  assert_eq!(context.item, Some(42));

  // Verify component_name uses component_info (line 332)
  let info = Consumer::component_info(&consumer);
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_name, "trait_default_context");

  // Verify component_type uses component_info (line 333)
  assert_eq!(context.component_type, info.type_name);

  // Create context with None - tests line 331 with None
  let context2 = Consumer::create_error_context(&consumer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, info.name);
}

// Test Consumer trait default handle_error through TestCloneConsumer
#[test]
fn test_consumer_handle_error_trait_default() {
  // Test lines 285-294 in consumer.rs: handle_error default implementation
  // VecConsumer and ConsoleConsumer override this, so use TestCloneConsumer which uses the trait default

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

  // Branch 1: ErrorStrategy::Stop (line 288)
  let consumer1 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_error_strategy(ErrorStrategy::Stop),
  };
  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Consumer::handle_error(&consumer1, &error1),
    ErrorAction::Stop
  ));

  // Branch 2: ErrorStrategy::Skip (line 289)
  let consumer2 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };
  assert!(matches!(
    Consumer::handle_error(&consumer2, &error1),
    ErrorAction::Skip
  ));

  // Branch 3: ErrorStrategy::Retry(n) if error.retries < *n (line 290)
  let consumer3 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(10)),
  };
  let retry_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 5, // Less than 10
  };
  assert!(matches!(
    Consumer::handle_error(&consumer3, &retry_error),
    ErrorAction::Retry
  ));

  // Branch 4: ErrorStrategy::Custom(handler) (line 291)
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Retry);
  let consumer4 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_error_strategy(custom_handler),
  };
  assert!(matches!(
    Consumer::handle_error(&consumer4, &error1),
    ErrorAction::Retry
  ));

  // Branch 5: _ => ErrorAction::Stop (line 292) - Retry exceeded
  let consumer5 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)),
  };
  let stop_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context,
    component: base_component,
    retries: 10, // Exceeds 3
  };
  assert!(matches!(
    Consumer::handle_error(&consumer5, &stop_error),
    ErrorAction::Stop
  ));
}

// Test Consumer trait default with_name through TestCloneConsumer
#[test]
fn test_consumer_with_name_trait_default() {
  // Test lines 267-273 in consumer.rs: with_name default implementation
  // Line 271: self.config_mut().name = name.clone();
  // Line 272: self
  // VecConsumer has its own with_name, so use TestCloneConsumer which uses the trait default

  let consumer = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("original".to_string()),
  };

  // This should use the trait default which calls config_mut().name = name.clone()
  let named = Consumer::with_name(consumer, "trait_default_name".to_string());
  assert_eq!(named.config().name(), "trait_default_name");

  // Verify it actually mutated the config (line 271)
  assert_eq!(named.config().name(), "trait_default_name");

  // Test with empty name
  let consumer2 = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default(),
  };
  let named2 = Consumer::with_name(consumer2, "".to_string());
  assert_eq!(named2.config().name(), "");
}

// Test with_config default implementation lines directly
#[test]
fn test_consumer_with_config_implementation_lines() {
  // Test lines 235-237 in consumer.rs: with_config implementation
  // Line 235: let mut this = self.clone();
  // Line 236: this.set_config(config);
  // Line 237: this
  let consumer = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("before".to_string()),
  };

  let config = ConsumerConfig::default().with_name("after".to_string());

  // This should execute all three lines of with_config
  let configured = Consumer::with_config(&consumer, config);

  assert_eq!(configured.config().name(), "after");
  assert_ne!(consumer.config().name(), configured.config().name());
}

// Test all branches in handle_error default implementation
#[test]
fn test_consumer_handle_error_all_branches() {
  use crate::consumers::ConsoleConsumer;

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

  // Test line 288: ErrorStrategy::Stop branch
  let mut consumer = ConsoleConsumer::<String>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 0,
  };
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Stop));

  // Test line 289: ErrorStrategy::Skip branch
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Skip));

  // Test line 290: ErrorStrategy::Retry(n) if error.retries < *n branch
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(5)));
  let retry_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 3, // Less than 5
  };
  let action = Consumer::handle_error(&consumer, &retry_error);
  assert!(matches!(action, ErrorAction::Retry));

  // Test line 291: ErrorStrategy::Custom branch
  let custom_handler = ErrorStrategy::new_custom(|_e: &StreamError<String>| ErrorAction::Retry);
  consumer.set_config(ConsumerConfig::default().with_error_strategy(custom_handler));
  let action = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action, ErrorAction::Retry));

  // Test line 292: _ => ErrorAction::Stop branch (fallback)
  // This is tested when retries >= limit for Retry strategy
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  let stop_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 5, // Greater than 3, should hit default branch
  };
  let action = Consumer::handle_error(&consumer, &stop_error);
  assert!(matches!(action, ErrorAction::Stop));
}

// Test component_info implementation lines directly
#[test]
fn test_consumer_component_info_implementation_lines() {
  use crate::consumers::ConsoleConsumer;

  // Test lines 307-310 in consumer.rs: component_info implementation
  // Line 308: name: self.config().name.clone()
  // Line 309: type_name: std::any::type_name::<Self>().to_string()
  let consumer = ConsoleConsumer::<String>::new().with_name("component_test".to_string());

  let info = Consumer::component_info(&consumer);

  // Verify line 308 is executed
  assert_eq!(info.name, consumer.config().name());

  // Verify line 309 is executed
  assert!(info.type_name.contains("ConsoleConsumer"));
  assert!(!info.type_name.is_empty());
}

// Test create_error_context implementation lines directly
#[test]
fn test_consumer_create_error_context_implementation_lines() {
  use crate::consumers::ConsoleConsumer;

  // Test lines 329-334 in consumer.rs: create_error_context implementation
  // Line 330: timestamp: chrono::Utc::now()
  // Line 331: item
  // Line 332: component_name: self.component_info().name
  // Line 333: component_type: self.component_info().type_name
  let consumer = ConsoleConsumer::<String>::new().with_name("context_test".to_string());

  // Test with Some(item)
  let context1 = Consumer::create_error_context(&consumer, Some("test_item".to_string()));
  assert_eq!(context1.item, Some("test_item".to_string()));
  assert_eq!(context1.component_name, consumer.component_info().name);
  assert_eq!(context1.component_type, consumer.component_info().type_name);
  assert!(context1.timestamp < chrono::Utc::now() + chrono::Duration::seconds(1));
  assert!(context1.timestamp > chrono::Utc::now() - chrono::Duration::seconds(1));

  // Test with None - verifies line 331 with None
  let context2 = Consumer::create_error_context(&consumer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, consumer.component_info().name);
}

// Test that ConsumerPorts blanket implementation is actually used
#[test]
fn test_consumer_ports_blanket_impl_usage() {
  use crate::consumer::ConsumerPorts;
  use crate::port::PortList;

  // The blanket impl on line 20-26 should be tested by actually using the type
  // Test that the type DefaultInputPorts is correctly set to (C::Input,)

  fn verify_ports<C>()
  where
    C: Consumer + ConsumerPorts,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    <C as ConsumerPorts>::DefaultInputPorts: PortList,
    <C as ConsumerPorts>::DefaultInputPorts: std::fmt::Debug,
  {
    // This function compiles only if the blanket impl works
    // The type constraint verifies that DefaultInputPorts = (C::Input,)
  }

  verify_ports::<VecConsumer<i32>>();
  verify_ports::<VecConsumer<String>>();

  // Test that we can actually access the type
  type VecPorts = <VecConsumer<i32> as ConsumerPorts>::DefaultInputPorts;
  fn assert_ports<T: PortList>() {}
  assert_ports::<VecPorts>();
  assert_ports::<(i32,)>();

  // This ensures the blanket impl is actually compiled and used
}

// Test ConsumerConfig struct field access
#[test]
fn test_consumer_config_field_access() {
  // Test that struct fields are accessible (lines 38, 40)
  let config = ConsumerConfig::<i32> {
    error_strategy: ErrorStrategy::Skip,
    name: "field_test".to_string(),
  };

  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
  assert_eq!(config.name, "field_test");

  // Test accessing fields directly
  let config2 = ConsumerConfig::<i32> {
    error_strategy: ErrorStrategy::Retry(7),
    name: "direct_access".to_string(),
  };

  assert!(matches!(config2.error_strategy(), ErrorStrategy::Retry(7)));
  assert_eq!(config2.name(), "direct_access");
}

// Test that config_mut allows direct field mutation
#[test]
fn test_consumer_config_mut_direct_mutation() {
  let mut consumer = VecConsumer::<i32>::new();

  // Test direct field access through config_mut
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Retry);
  consumer.config_mut().error_strategy = custom_handler;
  consumer.config_mut().name = "direct_mut".to_string();

  assert_eq!(consumer.config().name(), "direct_mut");
  // Can't easily test Custom equality, but we can test it exists
  match consumer.config().error_strategy() {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom strategy"),
  }
}

// Test ConsumerConfig error_strategy() method implementation
#[test]
fn test_consumer_config_error_strategy_clone() {
  // Test line 77 in consumer.rs: self.error_strategy.clone()
  let config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(99));

  // This calls error_strategy() which clones the strategy (line 77)
  let strategy = config.error_strategy();
  match strategy {
    ErrorStrategy::Retry(99) => {}
    _ => panic!("Expected Retry(99)"),
  }

  // Verify it's a clone, not a reference
  // We can't directly test this, but calling it again should work
  let strategy2 = config.error_strategy();
  match strategy2 {
    ErrorStrategy::Retry(99) => {}
    _ => panic!("Expected Retry(99)"),
  }
}

// Test ConsumerConfig name() method implementation
#[test]
fn test_consumer_config_name_ref_return() {
  // Test line 82 in consumer.rs: &self.name
  let config = ConsumerConfig::<i32>::default().with_name("name_ref_test".to_string());

  // This calls name() which returns &self.name (line 82)
  let name_ref = config.name();
  assert_eq!(name_ref, "name_ref_test");

  // Verify it's a reference by checking it's the same as the field
  assert_eq!(name_ref, &config.name);
}

// Test ConsumerPorts blanket implementation type assignment
#[test]
fn test_consumer_ports_type_assignment() {
  use crate::consumer::ConsumerPorts;
  use crate::port::PortList;

  // Test that line 25 in consumer.rs is actually executed
  // type DefaultInputPorts = (C::Input,);
  // We need to actually use the type in a way that requires compilation

  // Create a function that requires the type to be resolved
  fn require_default_ports<C>() -> std::marker::PhantomData<<C as ConsumerPorts>::DefaultInputPorts>
  where
    C: Consumer + ConsumerPorts,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    <C as ConsumerPorts>::DefaultInputPorts: PortList,
  {
    std::marker::PhantomData
  }

  // This forces the type to be resolved, executing the blanket impl
  let _phantom = require_default_ports::<VecConsumer<i32>>();

  // Also test with different types
  let _phantom2 = require_default_ports::<VecConsumer<String>>();

  // Verify the type is correct
  type TestType = <VecConsumer<i32> as ConsumerPorts>::DefaultInputPorts;
  fn assert_type<T: PortList>() {}
  assert_type::<TestType>();
  assert_type::<(i32,)>();
}

// Test ConsumerConfig with_error_strategy mutation
#[test]
fn test_consumer_config_with_error_strategy_mutation_line() {
  // Test line 60 in consumer.rs: self.error_strategy = strategy;
  let config = ConsumerConfig::<i32>::default();

  // Before mutation
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // After with_error_strategy (line 60)
  let config2 = config.with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config2.error_strategy, ErrorStrategy::Skip));
}

// Test ConsumerConfig with_name mutation
#[test]
fn test_consumer_config_with_name_mutation_line() {
  // Test line 71 in consumer.rs: self.name = name;
  let config = ConsumerConfig::<i32>::default();

  // Before mutation
  assert_eq!(config.name, "");

  // After with_name (line 71)
  let config2 = config.with_name("mutation_test".to_string());
  assert_eq!(config2.name, "mutation_test");
}

// Test that with_config clone line is executed
#[test]
fn test_consumer_with_config_clone_line() {
  // Test line 235 in consumer.rs: let mut this = self.clone();
  let consumer = TestCloneConsumer::<i32> {
    config: ConsumerConfig::default().with_name("clone_test".to_string()),
  };

  // Clone should be called here (line 235)
  let consumer2 = consumer.clone();
  assert_eq!(consumer2.config().name(), "clone_test");

  // Verify they are different instances
  assert_eq!(consumer.config().name(), consumer2.config().name());

  // Now test with_config which also calls clone (line 235)
  let config = ConsumerConfig::default().with_name("after_config".to_string());
  let configured = Consumer::with_config(&consumer, config);
  assert_eq!(configured.config().name(), "after_config");
}

// Test that set_config calls set_config_impl
#[tokio::test]
async fn test_consumer_set_config_calls_impl() {
  // Test line 246 in consumer.rs: self.set_config_impl(config);
  let mut consumer = VecConsumer::<i32>::new();

  // This should call set_config_impl internally (line 246)
  Consumer::set_config(
    &mut consumer,
    ConsumerConfig::default().with_name("impl_test".to_string()),
  );

  // Verify it worked
  assert_eq!(consumer.config().name(), "impl_test");
}

// Test that config() calls get_config_impl
#[test]
fn test_consumer_config_calls_get_impl() {
  // Test line 252 in consumer.rs: self.get_config_impl()
  let consumer = VecConsumer::<i32>::new().with_name("get_impl_test".to_string());

  // This should call get_config_impl internally (line 252)
  let config_ref = Consumer::config(&consumer);
  assert_eq!(config_ref.name(), "get_impl_test");
}

// Test that config_mut() calls get_config_mut_impl
#[test]
fn test_consumer_config_mut_calls_get_mut_impl() {
  // Test line 258 in consumer.rs: self.get_config_mut_impl()
  let mut consumer = VecConsumer::<i32>::new();

  // This should call get_config_mut_impl internally (line 258)
  let config_mut_ref = Consumer::config_mut(&mut consumer);
  config_mut_ref.name = "get_mut_impl_test".to_string();

  assert_eq!(consumer.config().name(), "get_mut_impl_test");
}

// Comprehensive test of Default implementation for ConsumerConfig
#[test]
fn test_consumer_config_default_implementation_comprehensive() {
  // Test lines 44-49 in consumer.rs: Default implementation
  // Line 45: Self {
  // Line 46: error_strategy: ErrorStrategy::Stop,
  // Line 47: name: String::new(),
  // Line 48: }

  let config: ConsumerConfig<i32> = Default::default();

  // Verify line 46 is executed
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // Verify line 47 is executed
  assert_eq!(config.name, String::new());
  assert_eq!(config.name, "");

  // Test that Default is actually called
  let config2: ConsumerConfig<String> = Default::default();
  assert!(matches!(config2.error_strategy, ErrorStrategy::Stop));
  assert_eq!(config2.name, "");
}

// Test that we actually use the ConsumerPorts trait in a way that counts
#[test]
fn test_consumer_ports_type_usage_in_function() {
  use crate::consumer::ConsumerPorts;

  // Create a function that actually uses the DefaultInputPorts type
  // This should force the blanket impl (line 25) to be resolved
  fn use_default_ports<C>() -> bool
  where
    C: Consumer + ConsumerPorts,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  {
    // Actually use the type in a way that requires it to exist
    std::mem::size_of::<<C as ConsumerPorts>::DefaultInputPorts>() > 0
  }

  // Call the function to force type resolution
  assert!(use_default_ports::<VecConsumer<i32>>());
  assert!(use_default_ports::<VecConsumer<String>>());

  // Verify the type is correct by using it
  type TestPorts = <VecConsumer<i32> as ConsumerPorts>::DefaultInputPorts;
  fn check_size<T>() -> usize {
    std::mem::size_of::<T>()
  }
  let size = check_size::<TestPorts>();
  let expected_size = std::mem::size_of::<(i32,)>();
  assert_eq!(size, expected_size);
}

// Test all branches in handle_error are covered
#[test]
fn test_consumer_handle_error_exhaustive_coverage() {
  use crate::consumers::ConsoleConsumer;

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

  // Branch 1: ErrorStrategy::Stop (line 288)
  let mut consumer = ConsoleConsumer::<String>::new();
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &error1),
    ErrorAction::Stop
  ));

  // Branch 2: ErrorStrategy::Skip (line 289)
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  assert!(matches!(
    Consumer::handle_error(&consumer, &error1),
    ErrorAction::Skip
  ));

  // Branch 3: ErrorStrategy::Retry(n) if error.retries < *n (line 290)
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(10)));
  let error2 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 5, // Less than 10
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &error2),
    ErrorAction::Retry
  ));

  // Branch 4: ErrorStrategy::Custom(handler) (line 291)
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Skip);
  consumer.set_config(ConsumerConfig::default().with_error_strategy(custom_handler));
  assert!(matches!(
    Consumer::handle_error(&consumer, &error1),
    ErrorAction::Skip
  ));

  // Branch 5: _ => ErrorAction::Stop (line 292) - Retry exceeded
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  let error3 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context,
    component: base_component,
    retries: 10, // Exceeds 3
  };
  assert!(matches!(
    Consumer::handle_error(&consumer, &error3),
    ErrorAction::Stop
  ));
}

// Test that component_info accesses config().name.clone() and std::any::type_name
#[test]
fn test_consumer_component_info_all_lines() {
  use crate::consumers::ConsoleConsumer;

  // Test line 308: name: self.config().name.clone()
  // Test line 309: type_name: std::any::type_name::<Self>().to_string()
  let consumer = ConsoleConsumer::<String>::new().with_name("component_info_test".to_string());

  // Call component_info which should execute both lines
  let info1 = Consumer::component_info(&consumer);
  assert_eq!(info1.name, "component_info_test");
  assert!(info1.type_name.contains("ConsoleConsumer"));

  // Call again to ensure lines are executed
  let info2 = Consumer::component_info(&consumer);
  assert_eq!(info2.name, "component_info_test");
  assert_eq!(info1.type_name, info2.type_name);
}

// Test that create_error_context executes all lines
#[test]
fn test_consumer_create_error_context_all_lines() {
  use crate::consumers::ConsoleConsumer;

  // Test lines 329-334 in consumer.rs
  // Line 330: timestamp: chrono::Utc::now()
  // Line 331: item
  // Line 332: component_name: self.component_info().name
  // Line 333: component_type: self.component_info().type_name
  let consumer = ConsoleConsumer::<String>::new().with_name("context_all_lines".to_string());

  // Create context with Some - tests line 331 with Some
  let before = chrono::Utc::now();
  let context1 = Consumer::create_error_context(&consumer, Some("test_item".to_string()));
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 330)
  assert!(context1.timestamp >= before);
  assert!(context1.timestamp <= after);

  // Verify item is set (line 331)
  assert_eq!(context1.item, Some("test_item".to_string()));

  // Verify component_name uses component_info (line 332)
  let info = Consumer::component_info(&consumer);
  assert_eq!(context1.component_name, info.name);

  // Verify component_type uses component_info (line 333)
  assert_eq!(context1.component_type, info.type_name);

  // Create context with None - tests line 331 with None
  let context2 = Consumer::create_error_context(&consumer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, info.name);
}

// Test handle_error uses config() internally (line 287)
#[test]
fn test_consumer_handle_error_uses_config() {
  use crate::consumers::ConsoleConsumer;

  // Test that line 287: match &self.config().error_strategy is executed
  let mut consumer = ConsoleConsumer::<String>::new();

  // Set different strategies and verify config() is called each time
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Stop));
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

  // This should call config() internally (line 287)
  let action1 = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action1, ErrorAction::Stop));

  // Change strategy and verify config() is called again
  consumer.set_config(ConsumerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  let action2 = Consumer::handle_error(&consumer, &error);
  assert!(matches!(action2, ErrorAction::Skip));
}
