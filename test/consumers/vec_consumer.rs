//! Tests for VecConsumer

use futures::stream;
use streamweave::consumers::VecConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[tokio::test]
async fn test_vec_consumer_basic() {
  let mut consumer = VecConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  consumer.consume(input_stream).await;

  assert_eq!(consumer.vec, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_vec_consumer_empty_stream() {
  let mut consumer = VecConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  consumer.consume(input_stream).await;

  assert_eq!(consumer.vec, Vec::<i32>::new());
}

#[tokio::test]
async fn test_vec_consumer_with_capacity() {
  let consumer = VecConsumer::<i32>::with_capacity(100);

  assert_eq!(consumer.vec.capacity(), 100);
}

#[tokio::test]
async fn test_vec_consumer_default() {
  let consumer = VecConsumer::<i32>::default();

  assert_eq!(consumer.vec, Vec::<i32>::new());
}

#[tokio::test]
async fn test_vec_consumer_with_name() {
  let consumer = VecConsumer::new().with_name("test_consumer".to_string());

  assert_eq!(consumer.config.name, "test_consumer");
}

#[tokio::test]
async fn test_vec_consumer_with_error_strategy() {
  let consumer = VecConsumer::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_vec_consumer_into_vec() {
  let mut consumer = VecConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![10, 20, 30]));

  consumer.consume(input_stream).await;

  let vec = consumer.into_vec();
  assert_eq!(vec, vec![10, 20, 30]);
}

#[tokio::test]
async fn test_vec_consumer_config_operations() {
  let mut consumer = VecConsumer::new();

  let config = consumer.get_config_impl();
  assert_eq!(config.name, "");

  let mut config = ConsumerConfig::default();
  config.name = "updated".to_string();
  consumer.set_config_impl(config);

  assert_eq!(consumer.get_config_impl().name, "updated");
}

#[tokio::test]
async fn test_vec_consumer_component_info() {
  let consumer = VecConsumer::new().with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("VecConsumer"));
}

#[tokio::test]
async fn test_vec_consumer_strings() {
  let mut consumer = VecConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![
    "a".to_string(),
    "b".to_string(),
    "c".to_string(),
  ]));

  consumer.consume(input_stream).await;

  assert_eq!(
    consumer.vec,
    vec!["a".to_string(), "b".to_string(), "c".to_string()]
  );
}
