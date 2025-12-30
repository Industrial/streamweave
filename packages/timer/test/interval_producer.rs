//! Tests for IntervalProducer

use futures::StreamExt;
use std::time::Duration;
use streamweave::Producer;
use streamweave_timer::IntervalProducer;

#[tokio::test]
async fn test_interval_producer_basic() {
  let mut producer = IntervalProducer::new(Duration::from_millis(10));
  let mut stream = producer.produce();

  // Collect first few items
  let first = stream.next().await;
  assert!(first.is_some());

  let second = stream.next().await;
  assert!(second.is_some());

  // Verify timestamps are increasing
  if let (Some(t1), Some(t2)) = (first, second) {
    assert!(t2 >= t1);
  }
}

#[tokio::test]
async fn test_interval_producer_with_name() {
  let producer =
    IntervalProducer::new(Duration::from_millis(10)).with_name("test_producer".to_string());

  assert_eq!(producer.config.name(), Some("test_producer"));
}

#[tokio::test]
async fn test_interval_producer_with_error_strategy() {
  use streamweave_error::ErrorStrategy;

  let producer =
    IntervalProducer::new(Duration::from_millis(10)).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config.error_strategy(),
    ErrorStrategy::Skip
  ));
}
