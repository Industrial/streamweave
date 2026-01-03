//! Tests for ArrayConsumer

use futures::stream;
use streamweave::consumers::ArrayConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[tokio::test]
async fn test_array_consumer_basic() {
  let mut consumer = ArrayConsumer::<i32, 5>::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  consumer.consume(input_stream).await;

  let array = consumer.into_array();
  assert_eq!(array[0], Some(1));
  assert_eq!(array[1], Some(2));
  assert_eq!(array[2], Some(3));
  assert_eq!(array[3], Some(4));
  assert_eq!(array[4], Some(5));
}

#[tokio::test]
async fn test_array_consumer_partial_fill() {
  let mut consumer = ArrayConsumer::<i32, 5>::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2]));

  consumer.consume(input_stream).await;

  let array = consumer.into_array();
  assert_eq!(array[0], Some(1));
  assert_eq!(array[1], Some(2));
  assert_eq!(array[2], None);
  assert_eq!(array[3], None);
  assert_eq!(array[4], None);
}

#[tokio::test]
async fn test_array_consumer_overflow() {
  let mut consumer = ArrayConsumer::<i32, 3>::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  consumer.consume(input_stream).await;

  let array = consumer.into_array();
  assert_eq!(array[0], Some(1));
  assert_eq!(array[1], Some(2));
  assert_eq!(array[2], Some(3));
  // Items 4 and 5 should be ignored
}

#[tokio::test]
async fn test_array_consumer_empty() {
  let mut consumer = ArrayConsumer::<i32, 3>::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  consumer.consume(input_stream).await;

  let array = consumer.into_array();
  assert_eq!(array[0], None);
  assert_eq!(array[1], None);
  assert_eq!(array[2], None);
}

#[tokio::test]
async fn test_array_consumer_default() {
  let consumer = ArrayConsumer::<i32, 3>::default();
  let array = consumer.into_array();

  assert_eq!(array[0], None);
  assert_eq!(array[1], None);
  assert_eq!(array[2], None);
}

#[tokio::test]
async fn test_array_consumer_with_name() {
  let consumer = ArrayConsumer::<i32, 3>::new().with_name("test".to_string());

  assert_eq!(consumer.config.name, "test");
}

#[tokio::test]
async fn test_array_consumer_with_error_strategy() {
  let consumer = ArrayConsumer::<i32, 3>::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_consumer_config_operations() {
  let mut consumer = ArrayConsumer::<i32, 3>::new();

  let mut config = ConsumerConfig::default();
  config.name = "updated".to_string();
  consumer.set_config_impl(config);

  assert_eq!(consumer.get_config_impl().name, "updated");
}

#[tokio::test]
async fn test_array_consumer_component_info() {
  let consumer = ArrayConsumer::<i32, 3>::new().with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("ArrayConsumer"));
}
