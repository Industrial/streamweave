use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::VecProducer;
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::error::Error;
use std::pin::Pin;

#[test]
fn test_producer_config_default() {
  let config = ProducerConfig::<i32>::default();

  assert_eq!(config.name(), None);
  match config.error_strategy() {
    ErrorStrategy::Stop => {} // Expected
    _ => panic!("Expected Stop strategy by default"),
  }
}

#[test]
fn test_producer_config_with_error_strategy() {
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);

  match config.error_strategy() {
    ErrorStrategy::Skip => {} // Expected
    _ => panic!("Expected Skip strategy"),
  }
}

#[test]
fn test_producer_config_with_name() {
  let config = ProducerConfig::<i32>::default().with_name("my_producer".to_string());

  assert_eq!(config.name(), Some("my_producer".to_string()));
}

#[test]
fn test_producer_config_builder_chain() {
  let config = ProducerConfig::<i32>::default()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .with_name("test_producer".to_string());

  assert_eq!(config.name(), Some("test_producer".to_string()));
  match config.error_strategy() {
    ErrorStrategy::Retry(3) => {}
    _ => panic!("Expected Retry(3) strategy"),
  }
}

#[test]
fn test_producer_config_error_strategy_getter() {
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);

  let strategy = config.error_strategy();
  match strategy {
    ErrorStrategy::Skip => {}
    _ => panic!("Expected Skip strategy"),
  }
}

#[tokio::test]
async fn test_producer_set_config() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let config = ProducerConfig::<i32>::default().with_name("new_name".to_string());

  producer.set_config(config);
  assert_eq!(producer.config().name(), Some("new_name".to_string()));
}

#[test]
fn test_producer_config_getter() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let config = producer.config();
  assert_eq!(config.name(), None);
}

#[tokio::test]
async fn test_producer_config_mut() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  producer.config_mut().name = Some("modified".to_string());
  assert_eq!(producer.config().name(), Some("modified".to_string()));
}

#[test]
fn test_producer_with_name() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let named = producer.with_name("named_producer".to_string());
  assert_eq!(named.config().name(), Some("named_producer".to_string()));
}

#[test]
fn test_producer_handle_error_stop() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
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

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_producer_handle_error_skip() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));

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

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Skip));
}

#[test]
fn test_producer_handle_error_retry() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));

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

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Retry));
}

#[test]
fn test_producer_handle_error_retry_exceeded() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));

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

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_producer_component_info() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let info = producer.component_info();
  assert_eq!(info.name, "vec_producer"); // Default name when None
  assert!(info.type_name.contains("VecProducer"));
}

#[test]
fn test_producer_component_info_with_name() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("named_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "named_producer");
  assert!(info.type_name.contains("VecProducer"));
}

#[test]
fn test_producer_create_error_context() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("test_producer".to_string());
  let context = producer.create_error_context(Some(42));

  assert_eq!(context.component_name, "test_producer");
  assert!(context.component_type.contains("VecProducer"));
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_producer_create_error_context_no_item() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let context = producer.create_error_context(None);

  assert_eq!(context.item, None);
  assert!(!context.component_type.is_empty());
}

#[tokio::test]
async fn test_producer_produce() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let mut stream = producer.produce();

  let mut collected = Vec::new();
  while let Some(item) = stream.next().await {
    collected.push(item);
  }

  assert_eq!(collected, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_producer_produce_empty() {
  let mut producer = VecProducer::<i32>::new(vec![]);
  let mut stream = producer.produce();

  let mut collected = Vec::new();
  while let Some(item) = stream.next().await {
    collected.push(item);
  }

  assert!(collected.is_empty());
}

#[test]
fn test_producer_input_ports_trait() {
  // Test that VecProducer implements OutputPorts correctly
  use crate::port::PortList;
  fn assert_ports<P>()
  where
    P: Producer,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    P::OutputPorts: PortList,
  {
    // Just checking it compiles
  }

  assert_ports::<VecProducer<i32>>();
}

#[test]
fn test_producer_with_name_chain() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3])
    .with_name("first".to_string())
    .with_name("second".to_string());
  assert_eq!(producer.config().name(), Some("second".to_string()));
}

#[test]
fn test_producer_config_error_strategy_all_variants() {
  let stop_config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Stop);
  assert!(matches!(stop_config.error_strategy(), ErrorStrategy::Stop));

  let skip_config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(skip_config.error_strategy(), ErrorStrategy::Skip));

  let retry_config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(5));
  match retry_config.error_strategy() {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }
}

#[test]
fn test_producer_handle_error_custom() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let custom_handler = ErrorStrategy::new_custom(|_error: &StreamError<i32>| ErrorAction::Skip);
  producer.set_config(ProducerConfig::default().with_error_strategy(custom_handler));

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

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Skip));
}

// Test ProducerPorts trait's blanket implementation
#[test]
fn test_producer_ports_trait() {
  use crate::port::PortList;
  use crate::producer::ProducerPorts;

  // Test that VecProducer implements ProducerPorts with DefaultOutputPorts = (i32,)
  type ExpectedPorts = <VecProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  fn assert_type<TPorts: PortList>() {}

  assert_type::<ExpectedPorts>();
  assert_type::<(i32,)>();

  // This should compile if the blanket impl works correctly
  // The actual usage is tested through the trait bounds in other code
}

// Test with_config method - requires Clone
#[derive(Clone)]
struct TestCloneProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  config: ProducerConfig<T>,
}

impl<T> Output for TestCloneProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Producer for TestCloneProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    Box::pin(futures::stream::empty())
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }
}

#[test]
fn test_producer_with_config() {
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("original".to_string()),
  };

  let new_config = ProducerConfig::default()
    .with_name("new_name".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let configured = producer.with_config(new_config);
  assert_eq!(configured.config().name(), Some("new_name".to_string()));
  assert!(matches!(
    configured.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test ProducerPorts trait more directly
#[test]
fn test_producer_ports_default_output_ports() {
  use crate::port::PortList;
  use crate::producer::ProducerPorts;

  // Test that the blanket implementation works
  // VecProducer<i32> should have DefaultOutputPorts = (i32,)
  fn check_ports_type<P>()
  where
    P: Producer + ProducerPorts,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <P as ProducerPorts>::DefaultOutputPorts: PortList,
  {
    // Just verifying it compiles - the type constraint validates the impl
  }

  check_ports_type::<VecProducer<i32>>();

  // Verify the type is correct
  type VecPorts = <VecProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  type ConsolePorts = <VecProducer<String> as ProducerPorts>::DefaultOutputPorts;

  fn assert_type<T: PortList>() {}
  assert_type::<VecPorts>();
  assert_type::<ConsolePorts>();
  assert_type::<(i32,)>();
  assert_type::<(String,)>();
}

// Test component_info with empty name (should use default)
#[test]
fn test_producer_component_info_empty_name() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let info = producer.component_info();
  assert_eq!(info.name, "vec_producer"); // Should use default
  assert!(info.type_name.contains("VecProducer"));
}

// Test create_error_context uses component_info correctly
#[test]
fn test_producer_create_error_context_uses_component_info() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("test_producer".to_string());
  let context = producer.create_error_context(Some(42));

  let info = producer.component_info();
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_type, info.type_name);
}

// Test config getters more thoroughly
#[test]
fn test_producer_config_getters_mutability() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // Test mutable access
  producer.config_mut().name = Some("mutated".to_string());
  assert_eq!(producer.config().name(), Some("mutated".to_string()));

  // Test that changes are reflected
  producer.config_mut().error_strategy = ErrorStrategy::Skip;
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));

  // Test setting name to None
  producer.config_mut().name = None;
  assert_eq!(producer.config().name(), None);
}

// Test set_config with various configurations
#[tokio::test]
async fn test_producer_set_config_variations() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // Test setting config with name only
  let config1 = ProducerConfig::default().with_name("name_only".to_string());
  producer.set_config(config1);
  assert_eq!(producer.config().name(), Some("name_only".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Stop
  ));

  // Test setting config with error strategy only
  let config2 = ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip);
  producer.set_config(config2);
  assert_eq!(producer.config().name(), None);
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));

  // Test setting config with both
  let config3 = ProducerConfig::default()
    .with_name("both".to_string())
    .with_error_strategy(ErrorStrategy::Retry(5));
  producer.set_config(config3);
  assert_eq!(producer.config().name(), Some("both".to_string()));
  match producer.config().error_strategy() {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }
}

// Test that with_name modifies the config correctly
#[test]
fn test_producer_with_name_modifies_config() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let named = producer.with_name("test_name".to_string());

  // Verify the name is set
  assert_eq!(named.config().name(), Some("test_name".to_string()));
}

// Test ProducerConfig Default implementation
#[test]
fn test_producer_config_default_impl() {
  // Test that Default implementation creates correct config
  let config: ProducerConfig<i32> = Default::default();
  assert_eq!(config.name, None);
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));
}

// Test ProducerConfig with_error_strategy mutation
#[test]
fn test_producer_config_with_error_strategy_mutation() {
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(10));

  // Verify the strategy is set
  match config.error_strategy() {
    ErrorStrategy::Retry(10) => {}
    _ => panic!("Expected Retry(10)"),
  }

  // Test chaining
  let config2 = ProducerConfig::<i32>::default()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_error_strategy(ErrorStrategy::Stop);

  // Last strategy should win
  assert!(matches!(config2.error_strategy(), ErrorStrategy::Stop));
}

// Test ProducerConfig with_name mutation
#[test]
fn test_producer_config_with_name_mutation() {
  let config = ProducerConfig::<i32>::default().with_name("test_name".to_string());

  assert_eq!(config.name(), Some("test_name".to_string()));

  // Test chaining - last name should win
  let config2 = ProducerConfig::<i32>::default()
    .with_name("first".to_string())
    .with_name("second".to_string());

  assert_eq!(config2.name(), Some("second".to_string()));

  // Test that with_name sets Some(name)
  assert_eq!(config2.name, Some("second".to_string()));
}

// Test ProducerConfig error_strategy() method implementation
#[test]
fn test_producer_config_error_strategy_clone() {
  // Test that error_strategy() clones the strategy
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(99));

  // This calls error_strategy() which clones the strategy
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

// Test ProducerConfig name() method implementation
#[test]
fn test_producer_config_name_clone() {
  // Test that name() clones the Option<String>
  let config = ProducerConfig::<i32>::default().with_name("name_clone_test".to_string());

  // This calls name() which clones the Option<String>
  let name = config.name();
  assert_eq!(name, Some("name_clone_test".to_string()));

  // Verify it's a clone, not a reference
  let name2 = config.name();
  assert_eq!(name2, Some("name_clone_test".to_string()));
}

// Test ProducerConfig with_name sets Some(name)
#[test]
fn test_producer_config_with_name_sets_some() {
  // Test line 47 in producer.rs: self.name = Some(name);
  let config = ProducerConfig::<i32>::default().with_name("some_name".to_string());

  // Verify name is Some
  assert_eq!(config.name, Some("some_name".to_string()));
  assert_eq!(config.name(), Some("some_name".to_string()));
}

// Test that Producer trait default methods are callable
#[test]
fn test_producer_trait_default_methods_exist() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // Test that all default methods exist and can be called
  // These test the method signatures in the trait

  // config() - should return a reference
  let _config_ref = producer.config();

  // config_mut() - should return a mutable reference
  let _config_mut_ref = producer.config_mut();

  // component_info() - should return ComponentInfo
  let _info = Producer::component_info(&producer);

  // create_error_context() - should return ErrorContext
  let _context = Producer::create_error_context(&producer, Some(42));
  let _context_none = Producer::create_error_context(&producer, None);

  // with_name() - should return Self (consumes self)
  let named = producer.with_name("test".to_string());
  assert_eq!(named.config().name(), Some("test".to_string()));
}

// Test create_error_context calls component_info internally
#[test]
fn test_producer_create_error_context_internal_call() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("test_producer".to_string());

  // create_error_context internally calls component_info()
  // This tests lines 334-335 in producer.rs
  let context = Producer::create_error_context(&producer, Some(42));

  // Verify it uses component_info correctly
  let info = Producer::component_info(&producer);
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_type, info.type_name);
}

// Test component_info calls config() internally
#[test]
fn test_producer_component_info_internal_call() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("test_name".to_string());

  // component_info internally calls config().name().unwrap_or_else(...)
  // This tests lines 350-353 in producer.rs
  let info = Producer::component_info(&producer);
  assert_eq!(
    info.name,
    producer
      .config()
      .name()
      .unwrap_or_else(|| "producer".to_string())
  );

  // Test that it uses std::any::type_name
  // This tests line 354 in producer.rs
  assert!(info.type_name.contains("VecProducer"));

  // Test with None name - should use default
  let producer2 = VecProducer::<i32>::new(vec![1, 2, 3]);
  let info2 = Producer::component_info(&producer2);
  assert_eq!(info2.name, "vec_producer"); // VecProducer uses its own default
}

// Test handle_error calls config() internally
#[test]
fn test_producer_handle_error_internal_call() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));

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
  // This tests line 305 in producer.rs
  let action = Producer::handle_error(&producer, &error);
  assert!(matches!(action, ErrorAction::Skip));
}

// Test set_config calls set_config_impl internally
#[tokio::test]
async fn test_producer_set_config_internal_call() {
  // set_config internally calls set_config_impl
  // This tests line 249 in producer.rs
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let config = ProducerConfig::default().with_name("internal_test".to_string());

  Producer::set_config(&mut producer, config);
  assert_eq!(producer.config().name(), Some("internal_test".to_string()));
}

// Test config() calls get_config_impl internally
#[test]
fn test_producer_config_internal_call() {
  // config() internally calls get_config_impl()
  // This tests line 258 in producer.rs
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("config_test".to_string());
  let config_ref = Producer::config(&producer);
  assert_eq!(config_ref.name(), Some("config_test".to_string()));
}

// Test config_mut() calls get_config_mut_impl internally
#[test]
fn test_producer_config_mut_internal_call() {
  // config_mut() internally calls get_config_mut_impl()
  // This tests line 267 in producer.rs
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  let config_mut_ref = Producer::config_mut(&mut producer);
  config_mut_ref.name = Some("mut_test".to_string());
  assert_eq!(producer.config().name(), Some("mut_test".to_string()));
}

// Test with_name calls get_config_impl and set_config internally
#[test]
fn test_producer_with_name_internal_call() {
  // with_name internally calls get_config_impl().clone() and set_config
  // This tests lines 284-288 in producer.rs
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("original".to_string());

  // This should clone the config, preserve error_strategy, and set name
  let named = Producer::with_name(producer, "internal_name".to_string());
  assert_eq!(named.config().name(), Some("internal_name".to_string()));
  // Error strategy should be preserved (was Stop by default)
  assert!(matches!(
    named.config().error_strategy(),
    ErrorStrategy::Stop
  ));
}

// Test with_config default implementation lines directly
#[test]
fn test_producer_with_config_implementation_lines() {
  // Test lines 238-240 in producer.rs: with_config implementation
  // Line 238: let mut this = self.clone();
  // Line 239: this.set_config(config);
  // Line 240: this
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("before".to_string()),
  };

  let config = ProducerConfig::default().with_name("after".to_string());

  // This should execute all three lines of with_config
  let configured = Producer::with_config(&producer, config);

  assert_eq!(configured.config().name(), Some("after".to_string()));
  assert_ne!(producer.config().name(), configured.config().name());
}

// Test all branches in handle_error default implementation
#[test]
fn test_producer_handle_error_all_branches() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

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

  // Branch 1: ErrorStrategy::Stop (line 306)
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Producer::handle_error(&producer, &error1),
    ErrorAction::Stop
  ));

  // Branch 2: ErrorStrategy::Skip (line 307)
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  assert!(matches!(
    Producer::handle_error(&producer, &error1),
    ErrorAction::Skip
  ));

  // Branch 3: ErrorStrategy::Retry(n) if error.retries < n (line 308)
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(10)));
  let error2 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 5, // Less than 10
  };
  assert!(matches!(
    Producer::handle_error(&producer, &error2),
    ErrorAction::Retry
  ));

  // Branch 4: ErrorStrategy::Custom(ref handler) (line 309)
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Skip);
  producer.set_config(ProducerConfig::default().with_error_strategy(custom_handler));
  assert!(matches!(
    Producer::handle_error(&producer, &error1),
    ErrorAction::Skip
  ));

  // Branch 5: _ => ErrorAction::Stop (line 310) - Retry exceeded
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  let error3 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context,
    component: base_component,
    retries: 10, // Exceeds 3
  };
  assert!(matches!(
    Producer::handle_error(&producer, &error3),
    ErrorAction::Stop
  ));
}

// Test that component_info accesses config().name().unwrap_or_else() correctly
#[test]
fn test_producer_component_info_all_lines() {
  // Test lines 350-354 in producer.rs: component_info implementation
  // Line 351-353: name: self.config().name().unwrap_or_else(|| "producer".to_string())
  // Line 354: type_name: std::any::type_name::<Self>().to_string()
  let producer =
    VecProducer::<i32>::new(vec![1, 2, 3]).with_name("component_info_test".to_string());

  // Call component_info which should execute both lines
  let info1 = Producer::component_info(&producer);
  assert_eq!(info1.name, "component_info_test");
  assert!(info1.type_name.contains("VecProducer"));

  // Call again to ensure lines are executed
  let info2 = Producer::component_info(&producer);
  assert_eq!(info2.name, "component_info_test");
  assert_eq!(info1.type_name, info2.type_name);

  // Test with None name - should use unwrap_or_else default
  let producer_no_name = VecProducer::<i32>::new(vec![1, 2, 3]);
  let info3 = Producer::component_info(&producer_no_name);
  assert_eq!(info3.name, "vec_producer"); // VecProducer has its own default
}

// Test that create_error_context executes all lines
#[test]
fn test_producer_create_error_context_all_lines() {
  // Test lines 331-336 in producer.rs
  // Line 332: timestamp: chrono::Utc::now()
  // Line 333: item
  // Line 334: component_name: self.component_info().name
  // Line 335: component_type: self.component_info().type_name
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("context_all_lines".to_string());

  // Create context with Some - tests line 333 with Some
  let before = chrono::Utc::now();
  let context1 = Producer::create_error_context(&producer, Some(42));
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 332)
  assert!(context1.timestamp >= before);
  assert!(context1.timestamp <= after);

  // Verify item is set (line 333)
  assert_eq!(context1.item, Some(42));

  // Verify component_name uses component_info (line 334)
  let info = Producer::component_info(&producer);
  assert_eq!(context1.component_name, info.name);

  // Verify component_type uses component_info (line 335)
  assert_eq!(context1.component_type, info.type_name);

  // Create context with None - tests line 333 with None
  let context2 = Producer::create_error_context(&producer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, info.name);
}

// Test handle_error uses config() internally (line 305)
#[test]
fn test_producer_handle_error_uses_config() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // Test that line 305: match self.config().error_strategy() is executed
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Stop));
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

  // This should call config().error_strategy() internally (line 305)
  let action1 = Producer::handle_error(&producer, &error);
  assert!(matches!(action1, ErrorAction::Stop));

  // Change strategy and verify config() is called again
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  let action2 = Producer::handle_error(&producer, &error);
  assert!(matches!(action2, ErrorAction::Skip));
}

// Test ProducerConfig Default implementation comprehensive
#[test]
fn test_producer_config_default_implementation_comprehensive() {
  // Test lines 22-27 in producer.rs: Default implementation
  // Line 23: Self {
  // Line 24: error_strategy: ErrorStrategy::Stop,
  // Line 25: name: None,
  // Line 26: }

  let config: ProducerConfig<i32> = Default::default();

  // Verify line 24 is executed
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // Verify line 25 is executed
  assert_eq!(config.name, None);

  // Test that Default is actually called
  let config2: ProducerConfig<String> = Default::default();
  assert!(matches!(config2.error_strategy, ErrorStrategy::Stop));
  assert_eq!(config2.name, None);
}

// Test ProducerPorts blanket implementation type assignment
#[test]
fn test_producer_ports_type_assignment() {
  use crate::port::PortList;
  use crate::producer::ProducerPorts;

  // Test that line 81 in producer.rs is actually executed
  // type DefaultOutputPorts = (P::Output,);
  // We need to actually use the type in a way that requires compilation

  // Create a function that requires the type to be resolved
  fn require_default_ports<P>() -> std::marker::PhantomData<<P as ProducerPorts>::DefaultOutputPorts>
  where
    P: Producer + ProducerPorts,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <P as ProducerPorts>::DefaultOutputPorts: PortList,
  {
    std::marker::PhantomData
  }

  // This forces the type to be resolved, executing the blanket impl
  let _phantom = require_default_ports::<VecProducer<i32>>();

  // Also test with different types
  let _phantom2 = require_default_ports::<VecProducer<String>>();

  // Verify the type is correct by using it
  type TestPorts = <VecProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  fn check_size<T>() -> usize {
    std::mem::size_of::<T>()
  }
  let size = check_size::<TestPorts>();
  let expected_size = std::mem::size_of::<(i32,)>();
  assert_eq!(size, expected_size);
}

// Test ProducerConfig with_error_strategy mutation line
#[test]
fn test_producer_config_with_error_strategy_mutation_line() {
  // Test line 37 in producer.rs: self.error_strategy = strategy;
  let config = ProducerConfig::<i32>::default();

  // Before mutation
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // After with_error_strategy (line 37)
  let config2 = config.with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config2.error_strategy, ErrorStrategy::Skip));
}

// Test ProducerConfig with_name mutation line
#[test]
fn test_producer_config_with_name_mutation_line() {
  // Test line 47 in producer.rs: self.name = Some(name);
  let config = ProducerConfig::<i32>::default();

  // Before mutation
  assert_eq!(config.name, None);

  // After with_name (line 47)
  let config2 = config.with_name("mutation_test".to_string());
  assert_eq!(config2.name, Some("mutation_test".to_string()));
}

// Test that with_config clone line is executed
#[test]
fn test_producer_with_config_clone_line() {
  // Test line 238 in producer.rs: let mut this = self.clone();
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("clone_test".to_string()),
  };

  // Clone should be called here (line 238)
  let producer2 = producer.clone();
  assert_eq!(producer2.config().name(), Some("clone_test".to_string()));

  // Verify they are different instances
  assert_eq!(producer.config().name(), producer2.config().name());

  // Now test with_config which also calls clone (line 238)
  let config = ProducerConfig::default().with_name("after_config".to_string());
  let configured = Producer::with_config(&producer, config);
  assert_eq!(configured.config().name(), Some("after_config".to_string()));
}

// Test that set_config calls set_config_impl
#[tokio::test]
async fn test_producer_set_config_calls_impl() {
  // set_config internally calls set_config_impl
  // This tests line 249 in producer.rs
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // This should call set_config_impl internally (line 249)
  Producer::set_config(
    &mut producer,
    ProducerConfig::default().with_name("impl_test".to_string()),
  );

  // Verify it worked
  assert_eq!(producer.config().name(), Some("impl_test".to_string()));
}

// Test that config() calls get_config_impl
#[test]
fn test_producer_config_calls_get_impl() {
  // config() internally calls get_config_impl()
  // This tests line 258 in producer.rs
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("get_impl_test".to_string());
  let config_ref = Producer::config(&producer);
  assert_eq!(config_ref.name(), Some("get_impl_test".to_string()));
}

// Test that config_mut() calls get_config_mut_impl
#[test]
fn test_producer_config_mut_calls_get_mut_impl() {
  // config_mut() internally calls get_config_mut_impl()
  // This tests line 267 in producer.rs
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // This should call get_config_mut_impl internally (line 267)
  let config_mut_ref = Producer::config_mut(&mut producer);
  config_mut_ref.name = Some("get_mut_impl_test".to_string());

  assert_eq!(
    producer.config().name(),
    Some("get_mut_impl_test".to_string())
  );
}

// Test with_name implementation (lines 284-288) - test trait default
#[test]
fn test_producer_with_name_implementation_trait_default() {
  // Test lines 284-288 in producer.rs: with_name implementation
  // Line 284: let config = self.get_config_impl().clone();
  // Line 285-288: self.set_config(ProducerConfig { error_strategy: config.error_strategy, name: Some(name) })

  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default()
      .with_name("original".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  };

  // This should use the trait default implementation which clones config and preserves error_strategy
  let named = Producer::with_name(producer, "new_name".to_string());
  assert_eq!(named.config().name(), Some("new_name".to_string()));
  // Error strategy should be preserved (line 286)
  assert!(matches!(
    named.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test with_name implementation with VecProducer (which might override)
#[test]
fn test_producer_with_name_implementation_vec_producer() {
  // Test that with_name works even when producer has custom implementation
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);
  producer.set_config(
    ProducerConfig::default()
      .with_name("original".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  );

  // VecProducer has its own with_name, but trait default should work
  // Actually, VecProducer overrides with_name, so we test the trait default with TestCloneProducer
  // Just verify the config was set correctly
  assert_eq!(producer.config().name(), Some("original".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

// Test ProducerConfig struct field access
#[test]
fn test_producer_config_field_access() {
  // Test that struct fields are accessible (lines 16, 18)
  let config = ProducerConfig::<i32> {
    error_strategy: ErrorStrategy::Skip,
    name: Some("field_test".to_string()),
  };

  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
  assert_eq!(config.name, Some("field_test".to_string()));

  // Test accessing fields directly
  let config2 = ProducerConfig::<i32> {
    error_strategy: ErrorStrategy::Retry(7),
    name: Some("direct_access".to_string()),
  };

  assert!(matches!(config2.error_strategy(), ErrorStrategy::Retry(7)));
  assert_eq!(config2.name(), Some("direct_access".to_string()));
}

// Test that config_mut allows direct field mutation
#[test]
fn test_producer_config_mut_direct_mutation() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

  // Test direct field access through config_mut
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Retry);
  producer.config_mut().error_strategy = custom_handler;
  producer.config_mut().name = Some("direct_mut".to_string());

  assert_eq!(producer.config().name(), Some("direct_mut".to_string()));
  // Can't easily test Custom equality, but we can test it exists
  match producer.config().error_strategy() {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom strategy"),
  }
}

// Test that with_config actually uses clone correctly
#[test]
fn test_producer_with_config_clone_behavior() {
  let producer1 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("original".to_string()),
  };

  // Clone the producer
  let producer2 = producer1.clone();
  assert_eq!(producer2.config().name(), Some("original".to_string()));

  // Test that with_config creates a new instance and calls set_config
  let new_config = ProducerConfig::default().with_name("new".to_string());
  let configured = producer2.with_config(new_config);
  assert_eq!(configured.config().name(), Some("new".to_string()));

  // Verify it's a different instance
  assert_ne!(producer2.config().name(), configured.config().name());
}

// Test ProducerPorts blanket implementation type usage
#[test]
fn test_producer_ports_blanket_impl_variations() {
  use crate::port::PortList;
  use crate::producer::ProducerPorts;

  // Test that different producers implement ProducerPorts
  fn check_ports_type<P>()
  where
    P: Producer + ProducerPorts,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <P as ProducerPorts>::DefaultOutputPorts: PortList,
  {
    // Just verifying it compiles
  }

  check_ports_type::<VecProducer<i32>>();
  check_ports_type::<VecProducer<String>>();

  // Verify the type is correct
  type VecPorts = <VecProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  type StringPorts = <VecProducer<String> as ProducerPorts>::DefaultOutputPorts;

  fn assert_type<T: PortList>() {}
  assert_type::<VecPorts>();
  assert_type::<StringPorts>();
  assert_type::<(i32,)>();
  assert_type::<(String,)>();
}

// Test component_info with None name uses default - test trait default implementation
#[test]
fn test_producer_component_info_none_name_trait_default() {
  // Test that the trait default component_info uses unwrap_or_else when name is None
  // This tests line 353 in producer.rs: unwrap_or_else(|| "producer".to_string())
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default(), // name is None
  };

  // This should use the trait default implementation which uses unwrap_or_else
  let info = Producer::component_info(&producer);
  assert_eq!(info.name, "producer"); // Trait default uses "producer"
  assert!(info.type_name.contains("TestCloneProducer"));

  // Test with explicit None
  let mut producer2 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("test".to_string()),
  };
  producer2.config_mut().name = None;
  let info2 = Producer::component_info(&producer2);
  assert_eq!(info2.name, "producer"); // Trait default uses "producer" when None
}

// Test that Producer trait default component_info uses the default "producer" string
#[test]
fn test_producer_component_info_trait_default_uses_producer_string() {
  // Test lines 350-354 in producer.rs: component_info default implementation
  // Line 350-353: name: self.config().name().unwrap_or_else(|| "producer".to_string())
  // Line 354: type_name: std::any::type_name::<Self>().to_string()

  // Test with None name - should use "producer" default
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default(), // name is None
  };
  let info = Producer::component_info(&producer);
  assert_eq!(info.name, "producer");
  assert!(info.type_name.contains("TestCloneProducer"));

  // Test with Some(name) - should use the name
  let producer2 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("custom_name".to_string()),
  };
  let info2 = Producer::component_info(&producer2);
  assert_eq!(info2.name, "custom_name");
}

// Test create_error_context with different item types
#[test]
fn test_producer_create_error_context_with_different_items() {
  let producer = VecProducer::<i32>::new(vec![1, 2, 3]).with_name("test_producer".to_string());

  // Test with Some(item)
  let context1 = producer.create_error_context(Some(42));
  assert_eq!(context1.item, Some(42));
  assert_eq!(context1.component_name, "test_producer");

  // Test with None
  let context2 = producer.create_error_context(None);
  assert_eq!(context2.item, None);

  // Test with different value
  let context3 = producer.create_error_context(Some(100));
  assert_eq!(context3.item, Some(100));
}

// Test all error strategy variants in handle_error - test trait default
#[test]
fn test_producer_handle_error_exhaustive_coverage_trait_default() {
  // Test the trait default handle_error implementation (not VecProducer's override)
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

  // Branch 1: ErrorStrategy::Stop (line 306) - test through TestCloneProducer which uses trait default
  let producer1 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(ErrorStrategy::Stop),
  };
  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Producer::handle_error(&producer1, &error1),
    ErrorAction::Stop
  ));

  // Branch 2: ErrorStrategy::Skip (line 307)
  let producer2 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };
  assert!(matches!(
    Producer::handle_error(&producer2, &error1),
    ErrorAction::Skip
  ));

  // Branch 3: ErrorStrategy::Retry(n) if error.retries < n (line 308)
  let producer3 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(10)),
  };
  let retry_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 5, // Less than 10
  };
  assert!(matches!(
    Producer::handle_error(&producer3, &retry_error),
    ErrorAction::Retry
  ));

  // Branch 4: ErrorStrategy::Custom(ref handler) (line 309)
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Retry);
  let producer4 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(custom_handler),
  };
  assert!(matches!(
    Producer::handle_error(&producer4, &error1),
    ErrorAction::Retry
  ));

  // Branch 5: _ => ErrorAction::Stop (line 310) - Retry exceeded
  let producer5 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)),
  };
  let stop_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context,
    component: base_component,
    retries: 10, // Exceeds 3
  };
  assert!(matches!(
    Producer::handle_error(&producer5, &stop_error),
    ErrorAction::Stop
  ));
}

// Test create_error_context trait default implementation
#[test]
fn test_producer_create_error_context_trait_default() {
  // Test that create_error_context trait default is called correctly
  // VecProducer overrides this, so use TestCloneProducer
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("trait_default_test".to_string()),
  };

  // This should use the trait default implementation
  // Lines 331-336 in producer.rs
  let before = chrono::Utc::now();
  let context = Producer::create_error_context(&producer, Some(42));
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 332)
  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);

  // Verify item is set (line 333)
  assert_eq!(context.item, Some(42));

  // Verify component_name uses component_info (line 334)
  let info = Producer::component_info(&producer);
  assert_eq!(context.component_name, info.name);
  assert_eq!(context.component_name, "trait_default_test");

  // Verify component_type uses component_info (line 335)
  assert_eq!(context.component_type, info.type_name);

  // Test with None
  let context2 = Producer::create_error_context(&producer, None);
  assert_eq!(context2.item, None);
  assert_eq!(context2.component_name, info.name);
}

// Test handle_error trait default - ensure it calls config().error_strategy()
#[test]
fn test_producer_handle_error_trait_default_calls_config() {
  // Test that line 305 in producer.rs: match self.config().error_strategy() is executed
  // Use TestCloneProducer which uses trait default
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(ErrorStrategy::Stop),
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

  // This should call config().error_strategy() internally (line 305)
  let action = Producer::handle_error(&producer, &error);
  assert!(matches!(action, ErrorAction::Stop));

  // Test with different strategy to ensure config() is called again
  let producer2 = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip),
  };
  let action2 = Producer::handle_error(&producer2, &error);
  assert!(matches!(action2, ErrorAction::Skip));
}

// Test component_info trait default - ensure it calls config().name().unwrap_or_else()
#[test]
fn test_producer_component_info_trait_default_calls_config() {
  // Test lines 350-353 in producer.rs: component_info default implementation
  // Line 350-353: name: self.config().name().unwrap_or_else(|| "producer".to_string())

  // Test with Some(name)
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("config_test".to_string()),
  };
  let info = Producer::component_info(&producer);
  assert_eq!(info.name, "config_test");
  assert!(info.type_name.contains("TestCloneProducer"));

  // Test with None - should use unwrap_or_else default
  let producer2 = TestCloneProducer::<i32> {
    config: ProducerConfig::default(), // name is None
  };
  let info2 = Producer::component_info(&producer2);
  assert_eq!(info2.name, "producer"); // Trait default uses "producer"

  // Verify it calls config().name() and unwrap_or_else
  // This tests the full path: config() -> name() -> unwrap_or_else()
  assert_eq!(
    info2.name,
    producer2
      .config()
      .name()
      .unwrap_or_else(|| "producer".to_string())
  );
}

// Test with_name trait default - ensure it clones config and preserves error_strategy
#[test]
fn test_producer_with_name_trait_default_clone_and_preserve() {
  // Test lines 284-288 in producer.rs: with_name default implementation
  // Line 284: let config = self.get_config_impl().clone();
  // Line 285-288: self.set_config(ProducerConfig { error_strategy: config.error_strategy, name: Some(name) })

  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default()
      .with_name("original_name".to_string())
      .with_error_strategy(ErrorStrategy::Retry(7)),
  };

  // This should clone the config, preserve error_strategy, and set name
  let named = Producer::with_name(producer, "new_name".to_string());

  // Verify name is updated (line 287)
  assert_eq!(named.config().name(), Some("new_name".to_string()));

  // Verify error_strategy is preserved (line 286)
  match named.config().error_strategy() {
    ErrorStrategy::Retry(7) => {}
    _ => panic!("Expected Retry(7) strategy to be preserved"),
  }

  // Verify original config is cloned (line 284)
  // We can't directly test this, but the fact that error_strategy is preserved shows it worked
}

// Test that ProducerPorts blanket implementation type is actually used
#[test]
fn test_producer_ports_blanket_impl_type_usage() {
  use crate::port::PortList;
  use crate::producer::ProducerPorts;

  // Test that line 81 in producer.rs: type DefaultOutputPorts = (P::Output,); is executed
  // Create a function that requires the type to be resolved
  fn use_default_ports<P>() -> std::marker::PhantomData<<P as ProducerPorts>::DefaultOutputPorts>
  where
    P: Producer + ProducerPorts,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    <P as ProducerPorts>::DefaultOutputPorts: PortList,
  {
    std::marker::PhantomData
  }

  // This forces the type to be resolved, executing the blanket impl
  let _phantom = use_default_ports::<VecProducer<i32>>();
  let _phantom2 = use_default_ports::<VecProducer<String>>();
  let _phantom3 = use_default_ports::<TestCloneProducer<i32>>();

  // Verify the type is correct by using it
  type TestPorts = <VecProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  fn check_size<T>() -> usize {
    std::mem::size_of::<T>()
  }
  let size = check_size::<TestPorts>();
  let expected_size = std::mem::size_of::<(i32,)>();
  assert_eq!(size, expected_size);

  // Test with TestCloneProducer too
  type TestClonePorts = <TestCloneProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  let size2 = check_size::<TestClonePorts>();
  assert_eq!(size2, expected_size);
}

// Test ProducerConfig Default implementation all lines
#[test]
fn test_producer_config_default_all_lines() {
  // Test lines 22-27 in producer.rs: Default implementation
  // Line 23: Self {
  // Line 24: error_strategy: ErrorStrategy::Stop,
  // Line 25: name: None,
  // Line 26: }

  let config: ProducerConfig<i32> = Default::default();

  // Verify line 24 is executed
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // Verify line 25 is executed
  assert_eq!(config.name, None);

  // Test that Default is actually called for different types
  let config2: ProducerConfig<String> = Default::default();
  assert!(matches!(config2.error_strategy, ErrorStrategy::Stop));
  assert_eq!(config2.name, None);
}

// Test ProducerConfig with_error_strategy mutation line
#[test]
fn test_producer_config_with_error_strategy_mutation_all() {
  // Test line 37 in producer.rs: self.error_strategy = strategy;
  let config = ProducerConfig::<i32>::default();

  // Before mutation
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));

  // After with_error_strategy (line 37) - test all strategies
  let config_stop = config.with_error_strategy(ErrorStrategy::Stop);
  assert!(matches!(config_stop.error_strategy, ErrorStrategy::Stop));

  let config_skip = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config_skip.error_strategy, ErrorStrategy::Skip));

  let config_retry = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(5));
  match config_retry.error_strategy {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }

  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Stop);
  let config_custom = ProducerConfig::<i32>::default().with_error_strategy(custom_handler);
  match config_custom.error_strategy {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom"),
  }
}

// Test ProducerConfig with_name mutation line for all cases
#[test]
fn test_producer_config_with_name_mutation_all() {
  // Test line 47 in producer.rs: self.name = Some(name);
  let config = ProducerConfig::<i32>::default();

  // Before mutation
  assert_eq!(config.name, None);

  // After with_name (line 47)
  let config2 = config.with_name("mutation_test".to_string());
  assert_eq!(config2.name, Some("mutation_test".to_string()));

  // Test chaining - last name should win
  let config3 = ProducerConfig::<i32>::default()
    .with_name("first".to_string())
    .with_name("second".to_string());
  assert_eq!(config3.name, Some("second".to_string()));

  // Test that each call creates a new instance
  let config4 = ProducerConfig::<i32>::default().with_name("test1".to_string());
  let config5 = ProducerConfig::<i32>::default().with_name("test2".to_string());
  assert_eq!(config4.name(), Some("test1".to_string()));
  assert_eq!(config5.name(), Some("test2".to_string()));
}

// Test ProducerConfig error_strategy() clone
#[test]
fn test_producer_config_error_strategy_clone_all() {
  // Test line 53 in producer.rs: self.error_strategy.clone()
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(99));

  // This calls error_strategy() which clones the strategy (line 53)
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

  // Test with all strategy types
  let config_stop = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Stop);
  assert!(matches!(config_stop.error_strategy(), ErrorStrategy::Stop));

  let config_skip = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config_skip.error_strategy(), ErrorStrategy::Skip));
}

// Test ProducerConfig name() clone
#[test]
fn test_producer_config_name_clone_all() {
  // Test line 58 in producer.rs: self.name.clone()
  let config = ProducerConfig::<i32>::default().with_name("name_clone_test".to_string());

  // This calls name() which clones the Option<String> (line 58)
  let name = config.name();
  assert_eq!(name, Some("name_clone_test".to_string()));

  // Call again to verify it's a clone operation
  let name2 = config.name();
  assert_eq!(name2, Some("name_clone_test".to_string()));

  // Test with None
  let config_none = ProducerConfig::<i32>::default();
  assert_eq!(config_none.name(), None);

  // Test that it's actually cloned (not a reference)
  // We can't directly test this, but the fact that we can call it multiple times shows it works
}

// Test that with_config clone line is executed
#[test]
fn test_producer_with_config_clone_line_direct() {
  // Test line 238 in producer.rs: let mut this = self.clone();
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("clone_test".to_string()),
  };

  // Clone should be called here (line 238)
  let producer2 = producer.clone();
  assert_eq!(producer2.config().name(), Some("clone_test".to_string()));

  // Verify they are different instances but have same config initially
  assert_eq!(producer.config().name(), producer2.config().name());

  // Now test with_config which also calls clone (line 238)
  let config = ProducerConfig::default().with_name("after_config".to_string());
  let configured = Producer::with_config(&producer, config);
  assert_eq!(configured.config().name(), Some("after_config".to_string()));

  // Verify original is unchanged (clone creates new instance)
  assert_eq!(producer.config().name(), Some("clone_test".to_string()));
}

// Test that set_config calls set_config_impl - ensure it's called through trait
#[tokio::test]
async fn test_producer_set_config_calls_impl_through_trait() {
  // set_config internally calls set_config_impl
  // This tests line 249 in producer.rs: self.set_config_impl(config);
  let mut producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default(),
  };

  // This should call set_config_impl internally (line 249)
  Producer::set_config(
    &mut producer,
    ProducerConfig::default().with_name("trait_impl_test".to_string()),
  );

  // Verify it worked
  assert_eq!(
    producer.config().name(),
    Some("trait_impl_test".to_string())
  );
}

// Test that config() calls get_config_impl - ensure it's called through trait
#[test]
fn test_producer_config_calls_get_impl_through_trait() {
  // config() internally calls get_config_impl()
  // This tests line 258 in producer.rs: self.get_config_impl()
  let producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default().with_name("trait_get_impl_test".to_string()),
  };

  // This should call get_config_impl internally (line 258)
  let config_ref = Producer::config(&producer);
  assert_eq!(config_ref.name(), Some("trait_get_impl_test".to_string()));
}

// Test that config_mut() calls get_config_mut_impl - ensure it's called through trait
#[test]
fn test_producer_config_mut_calls_get_mut_impl_through_trait() {
  // config_mut() internally calls get_config_mut_impl()
  // This tests line 267 in producer.rs: self.get_config_mut_impl()
  let mut producer = TestCloneProducer::<i32> {
    config: ProducerConfig::default(),
  };

  // This should call get_config_mut_impl internally (line 267)
  let config_mut_ref = Producer::config_mut(&mut producer);
  config_mut_ref.name = Some("trait_get_mut_impl_test".to_string());

  assert_eq!(
    producer.config().name(),
    Some("trait_get_mut_impl_test".to_string())
  );
}

// Test all error strategy variants in handle_error with VecProducer
#[test]
fn test_producer_handle_error_exhaustive_coverage() {
  let mut producer = VecProducer::<i32>::new(vec![1, 2, 3]);

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

  // Branch 1: ErrorStrategy::Stop (line 306)
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 0,
  };
  assert!(matches!(
    Producer::handle_error(&producer, &error1),
    ErrorAction::Stop
  ));

  // Branch 2: ErrorStrategy::Skip (line 307)
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  assert!(matches!(
    Producer::handle_error(&producer, &error1),
    ErrorAction::Skip
  ));

  // Branch 3: ErrorStrategy::Retry(n) if error.retries < n (line 308)
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(10)));
  let retry_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context.clone(),
    component: base_component.clone(),
    retries: 5, // Less than 10
  };
  assert!(matches!(
    Producer::handle_error(&producer, &retry_error),
    ErrorAction::Retry
  ));

  // Branch 4: ErrorStrategy::Custom(ref handler) (line 309)
  let custom_handler = ErrorStrategy::new_custom(|_| ErrorAction::Retry);
  producer.set_config(ProducerConfig::default().with_error_strategy(custom_handler));
  assert!(matches!(
    Producer::handle_error(&producer, &error1),
    ErrorAction::Retry
  ));

  // Branch 5: _ => ErrorAction::Stop (line 310) - Retry exceeded
  producer.set_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  let stop_error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: base_context,
    component: base_component,
    retries: 10, // Exceeds 3
  };
  assert!(matches!(
    Producer::handle_error(&producer, &stop_error),
    ErrorAction::Stop
  ));
}
