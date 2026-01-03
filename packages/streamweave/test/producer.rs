//! Tests for producer module

use futures::StreamExt;
use streamweave::error::ErrorStrategy;
use streamweave::output::Output;
use streamweave::producer::{Producer, ProducerConfig, ProducerPorts};
use streamweave::producers::VecProducer;

#[test]
fn test_producer_config_default() {
  let config = ProducerConfig::<i32>::default();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Stop));
  assert_eq!(config.name(), None);
}

#[test]
fn test_producer_config_with_error_strategy() {
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_producer_config_with_name() {
  let config = ProducerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config.name(), Some("test"));
}

#[test]
fn test_producer_config_error_strategy() {
  let config = ProducerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(3));
  assert!(matches!(config.error_strategy(), ErrorStrategy::Retry(3)));
}

#[test]
fn test_producer_config_name() {
  let config = ProducerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config.name(), Some("test"));
}

#[tokio::test]
async fn test_producer_trait_implementation() {
  let mut producer = VecProducer::new(vec![1, 2, 3]);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[test]
fn test_producer_ports_blanket_impl() {
  // Test that VecProducer implements ProducerPorts
  type VecProducerPorts = <VecProducer<i32> as ProducerPorts>::DefaultOutputPorts;
  // Should compile - this is a compile-time check
  assert!(true);
}
