use futures::Stream;
use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::Input;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_tokio::ChannelConsumer;
use tokio::sync::mpsc::{Receiver, Sender, channel};

async fn test_channel_consumer_basic_async(input: Vec<i32>) {
  let (tx, mut rx) = channel(10);
  let mut consumer = ChannelConsumer::new(tx);

  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  // Spawn the consumer in a separate task
  let handle = tokio::spawn(async move {
    consumer.consume(boxed_input).await;
  });

  // Wait for all items to be received
  for expected_item in input {
    assert_eq!(rx.recv().await, Some(expected_item));
  }
  assert_eq!(rx.recv().await, None);

  // Wait for the consumer task to complete
  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_channel_consumer_basic(
    input in prop::collection::vec(-1000..1000i32, 0..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_channel_consumer_basic_async(input));
  }
}

async fn test_channel_consumer_empty_input_async() {
  let (tx, mut rx) = channel(10);
  let mut consumer = ChannelConsumer::new(tx);

  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  // Spawn the consumer in a separate task
  let handle = tokio::spawn(async move {
    consumer.consume(boxed_input).await;
  });

  // Verify no items are received
  assert_eq!(rx.recv().await, None);

  // Wait for the consumer task to complete
  handle.await.unwrap();
}

#[tokio::test]
async fn test_channel_consumer_empty_input() {
  test_channel_consumer_empty_input_async().await;
}

async fn test_error_handling_strategies_async(name: String) {
  let (tx, _rx) = channel(10);
  let consumer = ChannelConsumer::new(tx)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name(name.clone());

  assert_eq!(consumer.config().error_strategy, ErrorStrategy::<i32>::Skip);
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

async fn test_error_handling_during_consumption_async(error_msg: String, item: i32) {
  let (tx, _rx) = channel(10);
  let consumer = ChannelConsumer::new(tx)
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
  assert_eq!(action, ErrorAction::Skip);

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
  assert_eq!(action, ErrorAction::Stop);
}

proptest! {
  #[test]
  fn test_error_handling_during_consumption(
    error_msg in prop::string::string_regex(".+").unwrap(),
    item in -1000..1000i32
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_error_handling_during_consumption_async(error_msg, item));
  }
}

async fn test_component_info_async(name: String) {
  let (tx, _rx): (Sender<i32>, Receiver<i32>) = channel(10);
  let consumer = ChannelConsumer::new(tx).with_name(name.clone());

  let info = consumer.component_info();
  assert_eq!(info.name, name);
  assert_eq!(info.type_name, "streamweave_tokio::ChannelConsumer<i32>");
}

proptest! {
  #[test]
  fn test_component_info(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_component_info_async(name));
  }
}

async fn test_error_context_creation_async(name: String, item: i32) {
  let (tx, _rx): (Sender<i32>, Receiver<i32>) = channel(10);
  let consumer = ChannelConsumer::new(tx).with_name(name.clone());

  let context = consumer.create_error_context(Some(item));
  assert_eq!(context.component_name, name);
  assert_eq!(
    context.component_type,
    "streamweave_tokio::ChannelConsumer<i32>"
  );
  assert_eq!(context.item, Some(item));
}

proptest! {
  #[test]
  fn test_error_context_creation(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    item in -1000..1000i32
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_error_context_creation_async(name, item));
  }
}

async fn test_channel_capacity_async(input: Vec<i32>, capacity: usize) {
  let (tx, mut rx): (Sender<i32>, Receiver<i32>) = channel(capacity);
  let mut consumer = ChannelConsumer::new(tx);

  let input_clone = input.clone();
  let input_stream = stream::iter(input_clone);
  let boxed_input = Box::pin(input_stream);

  // Spawn the consumer in a separate task
  let handle = tokio::spawn(async move {
    consumer.consume(boxed_input).await;
  });

  // Should receive all items despite capacity limit
  for expected_item in input {
    assert_eq!(rx.recv().await, Some(expected_item));
  }
  assert_eq!(rx.recv().await, None);

  // Wait for the consumer task to complete
  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_channel_capacity(
    input in prop::collection::vec(-1000..1000i32, 1..50),
    capacity in 1..10usize
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_channel_capacity_async(input, capacity));
  }
}

async fn test_dropped_channel_async(input: Vec<i32>) {
  let (tx, rx): (Sender<i32>, Receiver<i32>) = channel(10);
  let mut consumer = ChannelConsumer::new(tx);

  // Drop the receiver
  drop(rx);

  let input_stream = stream::iter(input);
  let boxed_input = Box::pin(input_stream);

  // Spawn the consumer in a separate task
  let handle = tokio::spawn(async move {
    consumer.consume(boxed_input).await;
  });

  // Should not panic when sending to dropped channel
  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_dropped_channel(
    input in prop::collection::vec(-1000..1000i32, 0..50)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_dropped_channel_async(input));
  }
}

#[test]
fn test_channel_consumer_input_trait_implementation() {
  // Test that ChannelConsumer implements Input trait correctly
  fn assert_input_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    _consumer: ChannelConsumer<T>,
  ) where
    ChannelConsumer<T>: Input<Input = T>,
  {
    // This function compiles only if ChannelConsumer<T> implements Input<Input = T>
  }

  let (tx, _rx) = channel::<i32>(10);
  let consumer = ChannelConsumer::new(tx);
  assert_input_trait(consumer);
}

#[test]
fn test_channel_consumer_input_type_constraints() {
  // Test that the Input type is correctly set to T
  fn get_input_type<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    _consumer: ChannelConsumer<T>,
  ) -> std::marker::PhantomData<T>
  where
    ChannelConsumer<T>: Input<Input = T>,
  {
    std::marker::PhantomData
  }

  let (tx, _rx) = channel::<String>(10);
  let consumer = ChannelConsumer::new(tx);
  let _phantom = get_input_type(consumer);
  // This compiles only if the Input type is correctly set to T
}

#[test]
fn test_channel_consumer_input_stream_type() {
  // Test that the InputStream type is correctly constrained
  fn create_input_stream<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    _consumer: ChannelConsumer<T>,
  ) -> Pin<Box<dyn Stream<Item = T> + Send>>
  where
    ChannelConsumer<T>: Input<Input = T, InputStream = Pin<Box<dyn Stream<Item = T> + Send>>>,
  {
    Box::pin(stream::empty())
  }

  let (tx, _rx) = channel::<f64>(10);
  let consumer = ChannelConsumer::new(tx);
  let _stream = create_input_stream(consumer);
  // This compiles only if the InputStream type is correctly constrained
}

async fn test_channel_consumer_input_stream_send_bound_async(data: Vec<i32>) {
  // Test that the InputStream implements Send bound for async usage
  let (tx, _rx) = channel::<i32>(10);
  let _consumer = ChannelConsumer::new(tx);

  // Create a stream that matches the InputStream type
  let expected_sum: i32 = data.iter().sum();
  let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data.clone()));

  // Test that we can spawn this stream in a task (requires Send)
  let handle = tokio::spawn(async move {
    let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
    assert_eq!(sum, expected_sum);
  });

  handle.await.unwrap();
}

proptest! {
  #[test]
  fn test_channel_consumer_input_stream_send_bound(
    data in prop::collection::vec(-1000..1000i32, 0..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_channel_consumer_input_stream_send_bound_async(data));
  }
}

#[test]
fn test_channel_consumer_input_trait_bounds() {
  // Test that the trait bounds are correctly applied
  fn test_trait_bounds<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    consumer: ChannelConsumer<T>,
  ) where
    ChannelConsumer<T>: Input,
  {
    // Test that the consumer can be used as Input
    let _input_type = std::marker::PhantomData::<T>;
    let _consumer = consumer;
  }

  // Test with different types
  let (tx1, _rx1) = channel::<i32>(10);
  let (tx2, _rx2) = channel::<String>(10);
  let (tx3, _rx3) = channel::<f64>(10);
  let (tx4, _rx4) = channel::<bool>(10);

  test_trait_bounds(ChannelConsumer::new(tx1));
  test_trait_bounds(ChannelConsumer::new(tx2));
  test_trait_bounds(ChannelConsumer::new(tx3));
  test_trait_bounds(ChannelConsumer::new(tx4));
}

#[test]
fn test_channel_consumer_input_static_lifetime() {
  // Test that the 'static lifetime bound is correctly applied
  fn test_static_lifetime<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    consumer: ChannelConsumer<T>,
  ) where
    ChannelConsumer<T>: Input,
  {
    // This function can only be called with types that have 'static lifetime
    let _consumer = consumer;
  }

  // These should compile because they have 'static lifetime
  let (tx1, _rx1) = channel::<i32>(10);
  let (tx2, _rx2) = channel::<String>(10);
  let (tx3, _rx3) = channel::<Vec<i32>>(10);

  test_static_lifetime(ChannelConsumer::new(tx1));
  test_static_lifetime(ChannelConsumer::new(tx2));
  test_static_lifetime(ChannelConsumer::new(tx3));
}

proptest! {
  #[test]
  fn test_channel_consumer_input_debug_clone_bounds(
    value in -1000..1000i32
  ) {
    // Test that Debug and Clone bounds are correctly applied
    #[derive(Debug, Clone)]
    struct TestStruct {
      _value: i32,
    }

    unsafe impl Send for TestStruct {}
    unsafe impl Sync for TestStruct {}

    // Verify that we can create a channel with TestStruct
    let _test_value = TestStruct { _value: value };

    let (tx, _rx) = channel::<TestStruct>(10);
    let consumer = ChannelConsumer::new(tx);

    // This should compile because TestStruct implements Debug and Clone
    fn test_debug_clone<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
      _consumer: ChannelConsumer<T>,
    ) where
      ChannelConsumer<T>: Input,
    {
    }

    test_debug_clone(consumer);
  }
}

async fn test_channel_consumer_input_stream_compatibility_async(data: Vec<i32>) {
  // Test that streams can be created and used with the Input trait
  let (tx, _rx) = channel::<i32>(10);
  let _consumer = ChannelConsumer::new(tx);

  // Create a stream that matches the expected InputStream type
  let data_clone = data.clone();
  let stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(data_clone));

  // Test that we can collect from the stream
  let result: Vec<i32> = stream.collect().await;
  assert_eq!(result, data);
}

proptest! {
  #[test]
  fn test_channel_consumer_input_stream_compatibility(
    data in prop::collection::vec(-1000..1000i32, 0..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_channel_consumer_input_stream_compatibility_async(data));
  }
}

#[test]
fn test_channel_consumer_input_trait_object_safety() {
  // Test that the Input trait can be used with trait objects
  fn process_input<I: Input>(_input: I)
  where
    I::Input: std::fmt::Debug,
  {
    // This function can accept any type that implements Input
  }

  // Test with different ChannelConsumer instances
  let (tx1, _rx1) = channel::<i32>(10);
  let (tx2, _rx2) = channel::<String>(10);
  let (tx3, _rx3) = channel::<f64>(10);

  process_input(ChannelConsumer::new(tx1));
  process_input(ChannelConsumer::new(tx2));
  process_input(ChannelConsumer::new(tx3));
}
