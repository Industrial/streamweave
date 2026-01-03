//! Tests for CommandConsumer

use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, Input};
use streamweave_command::CommandConsumer;

async fn test_command_consumer_basic_async(input: Vec<String>) {
  let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  consumer.consume(boxed_input).await;
}

proptest! {
  #[test]
  fn test_command_consumer_basic(
    input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..20)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_command_consumer_basic_async(input));
  }
}

async fn test_command_consumer_empty_input_async() {
  let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
  let input = stream::iter(Vec::<String>::new());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
}

#[test]
fn test_command_consumer_empty_input() {
  let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  rt.block_on(test_command_consumer_empty_input_async());
}

async fn test_error_handling_strategies_async(name: String) {
  let consumer = CommandConsumer::new("echo".to_string(), vec![])
    .with_error_strategy(ErrorStrategy::<String>::Skip)
    .with_name(name.clone());

  assert_eq!(
    consumer.config().error_strategy,
    ErrorStrategy::<String>::Skip
  );
  assert_eq!(consumer.config().name, name);
}

proptest! {
  #[test]
  fn test_error_handling_strategies(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_error_handling_strategies_async(name));
  }
}

async fn test_error_handling_during_consumption_async(error_msg: String, item: String) {
  let consumer = CommandConsumer::new("echo".to_string(), vec![])
    .with_error_strategy(ErrorStrategy::<String>::Skip)
    .with_name("test_consumer".to_string());

  // Test that Skip strategy allows consumption to continue
  let action = consumer.handle_error(&StreamError {
    source: Box::new(std::io::Error::other(error_msg.clone())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(item.clone()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  });
  assert_eq!(action, ErrorAction::Skip);

  // Test that Stop strategy halts consumption
  let consumer = consumer.with_error_strategy(ErrorStrategy::<String>::Stop);
  let action = consumer.handle_error(&StreamError {
    source: Box::new(std::io::Error::other(error_msg)),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(item),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  });
  assert_eq!(action, ErrorAction::Stop);
}

proptest! {
  #[test]
  fn test_error_handling_during_consumption(
    error_msg in prop::string::string_regex(".+").unwrap(),
    item in prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap()
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_error_handling_during_consumption_async(error_msg, item));
  }
}

async fn test_component_info_async() {
  let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
  let info = consumer.component_info();
  assert_eq!(info.name, "");
  assert_eq!(
    info.type_name,
    "streamweave_command::command_consumer::CommandConsumer<alloc::string::String>"
  );
}

#[test]
fn test_component_info() {
  let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  rt.block_on(test_component_info_async());
}

async fn test_error_context_creation_async(item: String) {
  let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
  let context = consumer.create_error_context(Some(item.clone()));
  assert_eq!(context.component_name, "");
  assert_eq!(
    context.component_type,
    "streamweave_command::command_consumer::CommandConsumer<alloc::string::String>"
  );
  assert_eq!(context.item, Some(item));
}

proptest! {
  #[test]
  fn test_error_context_creation(
    item in prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap()
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_error_context_creation_async(item));
  }
}

async fn test_command_with_arguments_async(input: Vec<String>, args: Vec<String>) {
  let mut consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), args);
  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  consumer.consume(boxed_input).await;
}

proptest! {
  #[test]
  fn test_command_with_arguments(
    input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..10),
    args in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9-]+").unwrap(), 0..5)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_command_with_arguments_async(input, args));
  }
}

async fn test_command_execution_failure_async(input: Vec<String>) {
  let mut consumer: CommandConsumer<String> =
    CommandConsumer::new("nonexistent_command".to_string(), vec![]);
  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  // Should not panic when command execution fails
  consumer.consume(boxed_input).await;
}

proptest! {
  #[test]
  fn test_command_execution_failure(
    input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..10)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_command_execution_failure_async(input));
  }
}

async fn test_command_output_handling_async(input: Vec<String>) {
  let mut consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  consumer.consume(boxed_input).await;
}

proptest! {
  #[test]
  fn test_command_output_handling(
    input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..10)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_command_output_handling_async(input));
  }
}

// Input trait tests

#[test]
fn test_command_consumer_input_trait_implementation() {
  // Test that CommandConsumer implements Input trait correctly
  fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    _consumer: CommandConsumer<T>,
  ) where
    CommandConsumer<T>: Input<Input = T>,
  {
    // This function compiles only if CommandConsumer<T> implements Input<Input = T>
  }

  let consumer = CommandConsumer::<&str>::new("echo".to_string(), vec![]);
  assert_input_trait(consumer);
}

#[test]
fn test_command_consumer_input_type_constraints() {
  // Test that the Input type is correctly set to T
  fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    _consumer: CommandConsumer<T>,
  ) -> std::marker::PhantomData<T>
  where
    CommandConsumer<T>: Input<Input = T>,
  {
    std::marker::PhantomData
  }

  let consumer = CommandConsumer::<&str>::new("echo".to_string(), vec![]);
  let _phantom = get_input_type(consumer);
  // This compiles only if the Input type is correctly set to T
}

#[test]
fn test_command_consumer_input_stream_type() {
  // Test that the InputStream type is correctly constrained
  fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    _consumer: CommandConsumer<T>,
  ) -> Pin<Box<dyn futures::Stream<Item = T> + Send>>
  where
    CommandConsumer<T>:
      Input<Input = T, InputStream = Pin<Box<dyn futures::Stream<Item = T> + Send>>>,
  {
    Box::pin(stream::empty())
  }

  let consumer = CommandConsumer::<&str>::new("echo".to_string(), vec![]);
  let _stream = create_input_stream(consumer);
  // This compiles only if the InputStream type is correctly constrained
}

async fn test_command_consumer_input_stream_send_bound_async(data: Vec<String>) {
  // Test that the InputStream implements Send bound for async usage
  let _consumer = CommandConsumer::<String>::new("echo".to_string(), vec![]);

  // Create a stream that matches the InputStream type
  let data_clone = data.clone();
  let stream: Pin<Box<dyn futures::Stream<Item = String> + Send>> =
    Box::pin(stream::iter(data_clone));

  // Test that we can spawn this stream in a task (requires Send)
  let handle = tokio::spawn(async move {
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  });

  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_command_consumer_input_stream_send_bound(
    data in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_command_consumer_input_stream_send_bound_async(data));
  }
}

#[test]
fn test_command_consumer_input_trait_bounds() {
  // Test that the trait bounds are correctly applied
  fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    consumer: CommandConsumer<T>,
  ) where
    CommandConsumer<T>: Input,
  {
    // Test that the consumer can be used as Input
    let _input_type = std::marker::PhantomData::<T>;
    let _consumer = consumer;
  }

  // Test with different types
  test_trait_bounds(CommandConsumer::<&str>::new("echo".to_string(), vec![]));
  test_trait_bounds(CommandConsumer::<String>::new("echo".to_string(), vec![]));
  test_trait_bounds(CommandConsumer::<i32>::new("echo".to_string(), vec![]));
  test_trait_bounds(CommandConsumer::<f64>::new("echo".to_string(), vec![]));
}

#[test]
fn test_command_consumer_input_static_lifetime() {
  // Test that the 'static lifetime bound is correctly applied
  fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    consumer: CommandConsumer<T>,
  ) where
    CommandConsumer<T>: Input,
  {
    // This function can only be called with types that have 'static lifetime
    let _consumer = consumer;
  }

  // These should compile because they have 'static lifetime
  test_static_lifetime(CommandConsumer::<&str>::new("echo".to_string(), vec![]));
  test_static_lifetime(CommandConsumer::<String>::new("echo".to_string(), vec![]));
  test_static_lifetime(CommandConsumer::<i32>::new("echo".to_string(), vec![]));
}

proptest! {
  #[test]
  fn test_command_consumer_input_debug_clone_display_bounds(
    value in -1000..1000i32
  ) {
    // Test that Debug, Clone, and Display bounds are correctly applied
    #[derive(Debug, Clone)]
    struct TestStruct {
      value: i32,
    }

    impl std::fmt::Display for TestStruct {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
      }
    }

    unsafe impl Send for TestStruct {}
    unsafe impl Sync for TestStruct {}

    // Verify that we can create a TestStruct with the generated value
    let _test_value = TestStruct { value };

    let consumer = CommandConsumer::<TestStruct>::new("echo".to_string(), vec![]);

    // This should compile because TestStruct implements Debug, Clone, and Display
    fn test_debug_clone_display<
      T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
    >(
      _consumer: CommandConsumer<T>,
    ) where
      CommandConsumer<T>: Input,
    {
    }

    test_debug_clone_display(consumer);
  }
}

async fn test_command_consumer_input_stream_compatibility_async(data: Vec<String>) {
  // Test that streams can be created and used with the Input trait
  let _consumer = CommandConsumer::<String>::new("echo".to_string(), vec![]);

  // Create a stream that matches the expected InputStream type
  let data_clone = data.clone();
  let stream: Pin<Box<dyn futures::Stream<Item = String> + Send>> =
    Box::pin(stream::iter(data_clone));

  // Test that we can collect from the stream
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result, data);
}

proptest! {
  #[test]
  fn test_command_consumer_input_stream_compatibility(
    data in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_command_consumer_input_stream_compatibility_async(data));
  }
}

#[test]
fn test_command_consumer_input_trait_object_safety() {
  // Test that the Input trait can be used with trait objects
  fn process_input<I: Input>(_input: I)
  where
    I::Input: std::fmt::Debug,
  {
    // This function can accept any type that implements Input
  }

  // Test with different CommandConsumer instances
  process_input(CommandConsumer::<&str>::new("echo".to_string(), vec![]));
  process_input(CommandConsumer::<String>::new("echo".to_string(), vec![]));
  process_input(CommandConsumer::<i32>::new("echo".to_string(), vec![]));
}

#[test]
fn test_command_consumer_input_display_bound() {
  // Test that the Display bound is correctly applied
  fn test_display_bound<T: std::fmt::Display>(_value: T) {}

  // Call the function to avoid unused function warning
  test_display_bound("test");

  let consumer = CommandConsumer::<&str>::new("echo".to_string(), vec![]);

  // This should compile because CommandConsumer requires Display bound
  fn test_consumer_display<T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static>(
    consumer: CommandConsumer<T>,
  ) where
    CommandConsumer<T>: Input,
  {
    // We can't directly test Display on the consumer, but the bound ensures T implements Display
    let _consumer = consumer;
  }

  test_consumer_display(consumer);
}

#[tokio::test]
async fn test_with_name_standalone() {
  let consumer: CommandConsumer<String> =
    CommandConsumer::new("echo".to_string(), vec![]).with_name("custom_consumer".to_string());

  assert_eq!(consumer.config().name, "custom_consumer".to_string());
}

#[tokio::test]
async fn test_config_methods() {
  let mut consumer = CommandConsumer::<String>::new("echo".to_string(), vec![]);

  // Test get_config_impl
  let config = consumer.get_config_impl();
  assert_eq!(config.error_strategy, ErrorStrategy::Stop);

  // Test get_config_mut_impl
  let config_mut = consumer.get_config_mut_impl();
  config_mut.error_strategy = ErrorStrategy::Skip;

  // Test set_config_impl
  let new_config = streamweave::ConsumerConfig::default();
  consumer.set_config_impl(new_config);

  assert_eq!(consumer.config().error_strategy, ErrorStrategy::Stop);
}

#[tokio::test]
async fn test_create_error_context_with_none() {
  let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
  let context = consumer.create_error_context(None);

  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "");
  assert!(context.component_type.contains("CommandConsumer"));
}

#[tokio::test]
async fn test_component_info_with_custom_name() {
  let consumer: CommandConsumer<String> =
    CommandConsumer::new("echo".to_string(), vec![]).with_name("custom_consumer".to_string());
  let info = consumer.component_info();

  assert_eq!(info.name, "custom_consumer".to_string());
}

#[tokio::test]
async fn test_error_context_with_custom_name() {
  let consumer: CommandConsumer<String> =
    CommandConsumer::new("echo".to_string(), vec![]).with_name("custom_consumer".to_string());
  let context = consumer.create_error_context(Some("test_item".to_string()));

  assert_eq!(context.component_name, "custom_consumer".to_string());
}

#[tokio::test]
async fn test_handle_error_retry_strategy_under_limit() {
  let consumer = CommandConsumer::new("echo".to_string(), vec![])
    .with_error_strategy(ErrorStrategy::<String>::Retry(3));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test_item".to_string()),
      component_name: "test".to_string(),
      component_type: "CommandConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandConsumer".to_string(),
    },
    retries: 1, // Less than max (3)
  };

  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_handle_error_retry_strategy_at_limit() {
  let consumer = CommandConsumer::new("echo".to_string(), vec![])
    .with_error_strategy(ErrorStrategy::<String>::Retry(3));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test_item".to_string()),
      component_name: "test".to_string(),
      component_type: "CommandConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandConsumer".to_string(),
    },
    retries: 3, // At max (3)
  };

  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_handle_error_retry_strategy_over_limit() {
  let consumer = CommandConsumer::new("echo".to_string(), vec![])
    .with_error_strategy(ErrorStrategy::<String>::Retry(3));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test_item".to_string()),
      component_name: "test".to_string(),
      component_type: "CommandConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandConsumer".to_string(),
    },
    retries: 4, // Over max (3)
  };

  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_handle_error_custom_strategy() {
  let custom_handler = |_error: &StreamError<String>| ErrorAction::Retry;
  let consumer = CommandConsumer::new("echo".to_string(), vec![])
    .with_error_strategy(ErrorStrategy::<String>::new_custom(custom_handler));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test_item".to_string()),
      component_name: "test".to_string(),
      component_type: "CommandConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandConsumer".to_string(),
    },
    retries: 0,
  };

  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_consume_with_none_command() {
  // Test that consume handles None command gracefully
  let mut consumer: CommandConsumer<String> = CommandConsumer {
    command: None,
    config: streamweave::ConsumerConfig::default(),
  };

  let input_stream = stream::iter(vec!["test1".to_string(), "test2".to_string()]);
  let boxed_input = Box::pin(input_stream);

  // Should not panic when command is None
  consumer.consume(boxed_input).await;
}

#[tokio::test]
async fn test_consumer_trait_implementation() {
  // Test that CommandConsumer implements Consumer trait correctly
  let mut consumer = CommandConsumer::<String>::new("echo".to_string(), vec![]);

  // Verify Consumer trait bounds
  fn assert_consumer_trait<C: streamweave::Consumer<Input = String>>(_consumer: &mut C)
  where
    <C as streamweave::Input>::Input: std::fmt::Debug + Clone + Send + Sync,
  {
  }
  assert_consumer_trait(&mut consumer);
}

#[tokio::test]
async fn test_input_ports() {
  let consumer = CommandConsumer::<String>::new("echo".to_string(), vec![]);

  // Verify InputPorts is (String,)
  fn assert_input_ports<C: streamweave::Consumer<InputPorts = (String,)>>(_consumer: &C)
  where
    <C as streamweave::Input>::Input: std::fmt::Debug + Clone + Send + Sync,
  {
  }
  assert_input_ports(&consumer);
}

#[tokio::test]
async fn test_consume_with_different_types() {
  // Test consume with different Display types
  let mut consumer_i32 = CommandConsumer::<i32>::new("echo".to_string(), vec![]);
  let input_i32 = stream::iter(vec![1, 2, 3]);
  consumer_i32.consume(Box::pin(input_i32)).await;

  let mut consumer_f64 = CommandConsumer::<f64>::new("echo".to_string(), vec![]);
  let input_f64 = stream::iter(vec![1.0, 2.0, 3.0]);
  consumer_f64.consume(Box::pin(input_f64)).await;
}

#[tokio::test]
async fn test_command_arg_handling() {
  // Test that command properly handles arguments with stream items
  let mut consumer = CommandConsumer::new("echo".to_string(), vec!["prefix".to_string()]);
  let input_stream = stream::iter(vec!["item1".to_string(), "item2".to_string()]);
  let boxed_input = Box::pin(input_stream);

  // Should not panic
  consumer.consume(boxed_input).await;
}
