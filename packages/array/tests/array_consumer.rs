//! Tests for ArrayConsumer

use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::{Consumer, Input};
use streamweave_array::ArrayConsumer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

proptest! {
  #[test]
  fn test_array_consumer_error_handling_strategies(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let consumer = ArrayConsumer::<i32, 3>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name(name.clone());

    prop_assert!(matches!(consumer.config().error_strategy, ErrorStrategy::<i32>::Skip));
    prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
  }

  #[test]
  fn test_error_handling_during_consumption(
    error_msg in prop::string::string_regex(".+").unwrap(),
    item in -1000..1000i32
  ) {
    let consumer = ArrayConsumer::<i32, 3>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    // Test that Skip strategy allows consumption to continue
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::other(error_msg.clone())),
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
    prop_assert_eq!(action, ErrorAction::Skip);

    // Test that Stop strategy halts consumption
    let consumer = consumer.with_error_strategy(ErrorStrategy::<i32>::Stop);
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
    prop_assert_eq!(action, ErrorAction::Stop);
  }
}

proptest! {
  #[test]
  fn test_component_info(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let consumer = ArrayConsumer::<i32, 3>::new().with_name(name.clone());

    let info = consumer.component_info();
    prop_assert_eq!(info.name.as_str(), name.as_str());
    prop_assert_eq!(
      info.type_name,
      "streamweave_array::array_consumer::ArrayConsumer<i32, 3>"
    );
  }

  #[test]
  fn test_error_context_creation(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    item in -1000..1000i32
  ) {
    let consumer = ArrayConsumer::<i32, 3>::new().with_name(name.clone());

    let context = consumer.create_error_context(Some(item));
    prop_assert_eq!(context.component_name.as_str(), name.as_str());
    prop_assert_eq!(
      context.component_type,
      "streamweave_array::array_consumer::ArrayConsumer<i32, 3>"
    );
    prop_assert_eq!(context.item, Some(item));
  }
}

async fn test_array_consumer_basic_async(input: Vec<i32>) {
  let mut consumer = ArrayConsumer::<i32, 3>::new();
  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();
  for (i, val) in input.iter().take(3).enumerate() {
    assert_eq!(array[i], Some(*val));
  }
}

#[test]
fn test_array_consumer_basic() {
  proptest::proptest!(|(input in prop::collection::vec(-1000..1000i32, 1..10))| {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_array_consumer_basic_async(input));
  });
}

#[tokio::test]
async fn test_array_consumer_empty_input() {
  let mut consumer = ArrayConsumer::<i32, 3>::new();
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();
  assert_eq!(array[0], None);
  assert_eq!(array[1], None);
  assert_eq!(array[2], None);
}

async fn test_array_consumer_capacity_exceeded_async(input: Vec<i32>, _capacity: usize) {
  let mut consumer = ArrayConsumer::<i32, 2>::new();
  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();
  // Should only store up to capacity (2)
  assert_eq!(array[0], input.first().copied());
  assert_eq!(array[1], input.get(1).copied());
}

#[test]
fn test_array_consumer_capacity_exceeded() {
  proptest::proptest!(|(input in prop::collection::vec(-1000..1000i32, 3..20))| {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_array_consumer_capacity_exceeded_async(input, 2));
  });
}

// ============================================================================
// Input Trait Tests for ArrayConsumer
// ============================================================================

#[test]
fn test_array_consumer_input_trait_implementation() {
  // Test that ArrayConsumer implements Input trait correctly
  fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
    _consumer: ArrayConsumer<T, N>,
  ) where
    ArrayConsumer<T, N>: Input<Input = T>,
  {
    // This function compiles only if ArrayConsumer<T, N> implements Input<Input = T>
  }

  let consumer = ArrayConsumer::<i32, 5>::new();
  assert_input_trait(consumer);
}

#[test]
fn test_array_consumer_input_type_constraints() {
  // Test that the Input type is correctly set to T
  fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
    _consumer: ArrayConsumer<T, N>,
  ) -> std::marker::PhantomData<T>
  where
    ArrayConsumer<T, N>: Input<Input = T>,
  {
    std::marker::PhantomData
  }

  let consumer = ArrayConsumer::<String, 3>::new();
  let _phantom = get_input_type(consumer);
  // This compiles only if the Input type is correctly set to T
}

#[test]
fn test_array_consumer_input_stream_type() {
  // Test that the InputStream type is correctly constrained
  fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
    _consumer: ArrayConsumer<T, N>,
  ) -> Pin<Box<dyn futures::Stream<Item = T> + Send>>
  where
    ArrayConsumer<T, N>:
      Input<Input = T, InputStream = Pin<Box<dyn futures::Stream<Item = T> + Send>>>,
  {
    Box::pin(stream::empty())
  }

  let consumer = ArrayConsumer::<f64, 2>::new();
  let _stream = create_input_stream(consumer);
  // This compiles only if the InputStream type is correctly constrained
}

#[tokio::test]
async fn test_array_consumer_input_stream_send_bound() {
  // Test that the InputStream implements Send bound for async usage
  let _consumer = ArrayConsumer::<i32, 4>::new();

  // Create a stream that matches the InputStream type
  let data = vec![1, 2, 3, 4];
  let stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> = Box::pin(stream::iter(data));

  // Test that we can spawn this stream in a task (requires Send)
  let handle = tokio::spawn(async move {
    let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
    assert_eq!(sum, 10);
  });

  handle.await.unwrap();
}

#[test]
fn test_array_consumer_input_trait_bounds() {
  // Test that the trait bounds are correctly applied
  fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
    consumer: ArrayConsumer<T, N>,
  ) where
    ArrayConsumer<T, N>: Input,
  {
    // Test that the consumer can be used as Input
    let _input_type = std::marker::PhantomData::<T>;
    let _consumer = consumer;
  }

  // Test with different types and sizes
  test_trait_bounds(ArrayConsumer::<i32, 1>::new());
  test_trait_bounds(ArrayConsumer::<String, 5>::new());
  test_trait_bounds(ArrayConsumer::<f64, 10>::new());
  test_trait_bounds(ArrayConsumer::<bool, 100>::new());
}

#[test]
fn test_array_consumer_input_static_lifetime() {
  // Test that the 'static lifetime bound is correctly applied
  fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
    consumer: ArrayConsumer<T, N>,
  ) where
    ArrayConsumer<T, N>: Input,
  {
    // This function can only be called with types that have 'static lifetime
    let _consumer = consumer;
  }

  // These should compile because they have 'static lifetime
  test_static_lifetime(ArrayConsumer::<i32, 3>::new());
  test_static_lifetime(ArrayConsumer::<String, 2>::new());
  test_static_lifetime(ArrayConsumer::<Vec<i32>, 1>::new());
}

#[test]
fn test_array_consumer_input_debug_clone_bounds() {
  // Test that Debug and Clone bounds are correctly applied
  #[derive(Debug, Clone)]
  struct TestStruct {
    _value: i32,
  }

  unsafe impl Send for TestStruct {}
  unsafe impl Sync for TestStruct {}

  let consumer = ArrayConsumer::<TestStruct, 3>::new();

  // This should compile because TestStruct implements Debug and Clone
  fn test_debug_clone<T: std::fmt::Debug + Clone + Send + Sync + 'static, const N: usize>(
    _consumer: ArrayConsumer<T, N>,
  ) where
    ArrayConsumer<T, N>: Input,
  {
  }

  test_debug_clone(consumer);
}

#[test]
fn test_array_consumer_input_different_sizes() {
  // Test that the Input trait works with different array sizes
  fn test_different_sizes<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    _consumer1: ArrayConsumer<T, 1>,
    _consumer2: ArrayConsumer<T, 5>,
    _consumer3: ArrayConsumer<T, 10>,
  ) where
    ArrayConsumer<T, 1>: Input<Input = T>,
    ArrayConsumer<T, 5>: Input<Input = T>,
    ArrayConsumer<T, 10>: Input<Input = T>,
  {
    // All consumers should have the same Input type T
  }

  test_different_sizes(
    ArrayConsumer::<i32, 1>::new(),
    ArrayConsumer::<i32, 5>::new(),
    ArrayConsumer::<i32, 10>::new(),
  );
}

#[tokio::test]
async fn test_array_consumer_input_stream_compatibility() {
  // Test that streams can be created and used with the Input trait
  let _consumer = ArrayConsumer::<i32, 3>::new();

  // Create a stream that matches the expected InputStream type
  let data = vec![1, 2, 3];
  let stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> = Box::pin(stream::iter(data));

  // Test that we can collect from the stream
  let result: Vec<i32> = stream.collect().await;
  assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_array_consumer_input_trait_object_safety() {
  // Test that the Input trait can be used with trait objects
  fn process_input<I: Input>(_input: I)
  where
    I::Input: std::fmt::Debug,
  {
    // This function can accept any type that implements Input
  }

  // Test with different ArrayConsumer instances
  process_input(ArrayConsumer::<i32, 3>::new());
  process_input(ArrayConsumer::<String, 5>::new());
  process_input(ArrayConsumer::<f64, 1>::new());
}

#[test]
fn test_array_consumer_default() {
  let consumer1 = ArrayConsumer::<i32, 3>::new();
  let consumer2 = ArrayConsumer::<i32, 3>::default();

  // Both should create equivalent consumers
  assert_eq!(consumer1.index, consumer2.index);
  assert_eq!(consumer1.array.len(), consumer2.array.len());
}

#[test]
fn test_error_handling_retry() {
  let consumer = ArrayConsumer::<i32, 3>::new().with_error_strategy(ErrorStrategy::Retry(3));

  // Test retry when retries < max
  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 2, // Less than max (3)
  };

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Retry));

  // Test stop when retries >= max
  let error_max_retries = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 3, // Equal to max
  };

  let action = consumer.handle_error(&error_max_retries);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_error_handling_custom() {
  let consumer = ArrayConsumer::<i32, 3>::new()
    .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  let action = consumer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Skip));
}

#[test]
fn test_create_error_context_without_item() {
  let consumer = ArrayConsumer::<i32, 3>::new().with_name("test_consumer".to_string());

  let context = consumer.create_error_context(None);
  assert_eq!(context.component_name, "test_consumer");
  assert_eq!(context.item, None);
  assert!(context.component_type.contains("ArrayConsumer"));
}

#[test]
fn test_create_error_context_default_name() {
  let consumer = ArrayConsumer::<i32, 3>::new();
  // No name set, should use empty string from config

  let context = consumer.create_error_context(Some(42));
  assert_eq!(context.component_name, ""); // Default ConsumerConfig has empty name
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_component_info_default_name() {
  let consumer = ArrayConsumer::<i32, 3>::new();
  // No name set, should use empty string from config

  let info = consumer.component_info();
  assert_eq!(info.name, ""); // Default ConsumerConfig has empty name
  assert!(info.type_name.contains("ArrayConsumer"));
}

#[test]
fn test_set_config_impl() {
  use streamweave::ConsumerConfig;

  let mut consumer = ArrayConsumer::<i32, 3>::new();
  let new_config = ConsumerConfig::<i32> {
    error_strategy: ErrorStrategy::Skip,
    name: "custom_name".to_string(),
  };

  consumer.set_config_impl(new_config.clone());

  let retrieved_config = consumer.get_config_impl();
  assert_eq!(retrieved_config.error_strategy, ErrorStrategy::<i32>::Skip);
  assert_eq!(retrieved_config.name, "custom_name");
}

#[test]
fn test_get_config_mut_impl() {
  let mut consumer = ArrayConsumer::<i32, 3>::new();

  let config_mut = consumer.get_config_mut_impl();
  config_mut.error_strategy = ErrorStrategy::Skip;
  config_mut.name = "mutated_name".to_string();

  let config = consumer.get_config_impl();
  assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name, "mutated_name");
}

#[tokio::test]
async fn test_consume_exact_capacity() {
  let mut consumer = ArrayConsumer::<i32, 3>::new();
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();

  assert_eq!(array[0], Some(1));
  assert_eq!(array[1], Some(2));
  assert_eq!(array[2], Some(3));
}

#[tokio::test]
async fn test_consume_single_item() {
  let mut consumer = ArrayConsumer::<i32, 5>::new();
  let input = stream::iter(vec![42]);
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();

  assert_eq!(array[0], Some(42));
  assert_eq!(array[1], None);
  assert_eq!(array[2], None);
  assert_eq!(array[3], None);
  assert_eq!(array[4], None);
}

#[tokio::test]
async fn test_consume_with_different_types() {
  // Test with String
  let mut consumer = ArrayConsumer::<String, 2>::new();
  let input = stream::iter(vec!["hello".to_string(), "world".to_string()]);
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();

  assert_eq!(array[0], Some("hello".to_string()));
  assert_eq!(array[1], Some("world".to_string()));

  // Test with bool
  let mut consumer = ArrayConsumer::<bool, 3>::new();
  let input = stream::iter(vec![true, false, true]);
  let boxed_input = Box::pin(input);

  consumer.consume(boxed_input).await;
  let array = consumer.into_array();

  assert_eq!(array[0], Some(true));
  assert_eq!(array[1], Some(false));
  assert_eq!(array[2], Some(true));
}

#[test]
fn test_consumer_input_ports() {
  use streamweave::{Consumer, ConsumerPorts};

  let consumer = ArrayConsumer::<i32, 3>::new();
  // Verify InputPorts is correctly set - this is a compile-time check
  // If ConsumerPorts is not implemented, this won't compile
  fn assert_input_ports<C: Consumer<Input = i32> + ConsumerPorts>(_consumer: C) {
    // This compiles only if ConsumerPorts is implemented
    // The associated type DefaultInputPorts should be (i32,)
  }

  assert_input_ports(consumer);
}
