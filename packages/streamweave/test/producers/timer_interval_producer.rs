//! Tests for TimerIntervalProducer

use futures::StreamExt;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::producers::TimerIntervalProducer;
use streamweave::{Producer, ProducerConfig};

#[tokio::test]
async fn test_timer_interval_producer_basic() {
  let mut producer = TimerIntervalProducer::new(Duration::from_millis(10));
  let mut stream = producer.produce();

  // Take first few items
  let start = std::time::Instant::now();
  let item1 = stream.next().await;
  let item2 = stream.next().await;

  assert!(item1.is_some());
  assert!(item2.is_some());

  // Verify timing (should be approximately 10ms apart)
  let elapsed = start.elapsed();
  assert!(elapsed >= Duration::from_millis(8)); // Allow some tolerance
}

#[tokio::test]
async fn test_timer_interval_producer_with_name() {
  let producer = TimerIntervalProducer::new(Duration::from_secs(1)).with_name("timer".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("timer"));
}

#[tokio::test]
async fn test_timer_interval_producer_with_error_strategy() {
  let producer =
    TimerIntervalProducer::new(Duration::from_secs(1)).with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_timer_interval_producer_config_operations() {
  let mut producer = TimerIntervalProducer::new(Duration::from_secs(1));

  let config = producer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(producer.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_timer_interval_producer_component_info() {
  let producer = TimerIntervalProducer::new(Duration::from_secs(1)).with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("TimerIntervalProducer"));
}
