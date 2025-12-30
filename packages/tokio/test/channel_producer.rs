use futures::StreamExt;
use proptest::prelude::*;
use streamweave::Output;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_tokio::ChannelProducer;
use tokio::sync::mpsc::channel;

async fn test_channel_producer_basic_async(input: Vec<i32>) {
  let (tx, rx) = channel(10);
  let mut producer = ChannelProducer::new(rx);

  // Send items to the channel
  for item in &input {
    tx.send(*item).await.unwrap();
  }
  drop(tx); // Close the sender to signal end of stream

  // Get the stream from producer
  let mut stream = producer.produce();

  // Collect all items
  let mut received = Vec::new();
  while let Some(item) = stream.next().await {
    received.push(item);
  }

  assert_eq!(received, input);
}

proptest! {
  #[test]
  fn test_channel_producer_basic(
    input in prop::collection::vec(-1000..1000i32, 0..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_channel_producer_basic_async(input));
  }
}

async fn test_channel_producer_empty_input_async() {
  let (tx, rx) = channel::<i32>(10);
  let mut producer = ChannelProducer::new(rx);

  // Close the sender immediately
  drop(tx);

  // Get the stream from producer
  let mut stream = producer.produce();

  // Should receive nothing
  assert_eq!(stream.next().await, None);
}

#[tokio::test]
async fn test_channel_producer_empty_input() {
  test_channel_producer_empty_input_async().await;
}

async fn test_error_handling_strategies_async(name: String) {
  let (_, rx) = channel(10);
  let producer = ChannelProducer::new(rx)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name(name.clone());

  assert_eq!(
    producer.config().error_strategy(),
    &ErrorStrategy::<i32>::Skip
  );
  assert_eq!(producer.config().name(), Some(&name));
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

async fn test_error_handling_during_production_async(error_msg: String, item: i32) {
  let (_, rx) = channel(10);
  let producer = ChannelProducer::new(rx)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_producer".to_string());

  // Test that Skip strategy allows production to continue
  let action = producer.handle_error(&StreamError {
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

  // Test that Stop strategy halts production
  let producer = producer.with_error_strategy(ErrorStrategy::<i32>::Stop);
  let action = producer.handle_error(&StreamError {
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
  fn test_error_handling_during_production(
    error_msg in prop::string::string_regex(".+").unwrap(),
    item in -1000..1000i32
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_error_handling_during_production_async(error_msg, item));
  }
}

async fn test_component_info_async(name: String) {
  let (_, rx) = channel(10);
  let producer = ChannelProducer::new(rx).with_name(name.clone());

  let info = producer.component_info();
  assert_eq!(info.name, name);
  assert_eq!(info.type_name, "streamweave_tokio::ChannelProducer<i32>");
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
  let (_, rx) = channel(10);
  let producer = ChannelProducer::new(rx).with_name(name.clone());

  let context = producer.create_error_context(Some(item));
  assert_eq!(context.component_name, name);
  assert_eq!(
    context.component_type,
    "streamweave_tokio::ChannelProducer<i32>"
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

#[test]
fn test_channel_producer_output_trait_implementation() {
  // Test that ChannelProducer implements Output trait correctly
  fn assert_output_trait<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    _producer: ChannelProducer<T>,
  ) where
    ChannelProducer<T>: Output<Output = T>,
  {
    // This function compiles only if ChannelProducer<T> implements Output<Output = T>
  }

  let (_, rx) = channel::<i32>(10);
  let producer = ChannelProducer::new(rx);
  assert_output_trait(producer);
}

#[test]
fn test_channel_producer_output_type_constraints() {
  // Test that the Output type is correctly set to T
  fn get_output_type<T: std::fmt::Debug + Clone + Send + Sync + 'static>(
    _producer: ChannelProducer<T>,
  ) -> std::marker::PhantomData<T>
  where
    ChannelProducer<T>: Output<Output = T>,
  {
    std::marker::PhantomData
  }

  let (_, rx) = channel::<String>(10);
  let producer = ChannelProducer::new(rx);
  let _phantom = get_output_type(producer);
  // This compiles only if the Output type is correctly set to T
}

#[tokio::test]
#[should_panic(expected = "Receiver already consumed")]
async fn test_channel_producer_double_consume() {
  let (_, rx) = channel::<i32>(10);
  let mut producer = ChannelProducer::new(rx);

  // First consume should work
  let _stream1 = producer.produce();

  // Second consume should panic
  let _stream2 = producer.produce();
}
