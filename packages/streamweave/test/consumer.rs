//! Tests for consumer module

use futures::StreamExt;
use futures::stream;
use streamweave::consumer::{Consumer, ConsumerConfig, ConsumerPorts};
use streamweave::consumers::VecConsumer;
use streamweave::error::{ErrorAction, ErrorStrategy};
use streamweave::input::Input;

#[test]
fn test_consumer_config_default() {
  let config = ConsumerConfig::<i32>::default();
  assert!(matches!(config.error_strategy, ErrorStrategy::Stop));
  assert_eq!(config.name, "");
}

#[test]
fn test_consumer_config_with_error_strategy() {
  let config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[test]
fn test_consumer_config_with_name() {
  let config = ConsumerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config.name, "test");
}

#[test]
fn test_consumer_config_error_strategy() {
  let config = ConsumerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(3));
  assert!(matches!(config.error_strategy(), ErrorStrategy::Retry(3)));
}

#[test]
fn test_consumer_config_name() {
  let config = ConsumerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config.name(), "test");
}

#[tokio::test]
async fn test_consumer_trait_implementation() {
  let mut consumer = VecConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  consumer.consume(input_stream).await;

  assert_eq!(consumer.vec, vec![1, 2, 3]);
}

#[test]
fn test_consumer_ports_blanket_impl() {
  // Test that VecConsumer implements ConsumerPorts
  type VecConsumerPorts = <VecConsumer<i32> as ConsumerPorts>::DefaultInputPorts;
  // Should compile - this is a compile-time check
  assert!(true);
}
