//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use std::time::Duration;
use streamweave_pipeline::PipelineBuilder;
use streamweave_timer::IntervalProducer;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

/// Test: Interval Producer
///
/// This test recreates the "Interval Producer" example from README.md.
#[tokio::test]
async fn test_interval_producer_example() {
  // Example from README.md lines 33-47
  use std::time::Duration;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_timer::IntervalProducer;

  let producer = IntervalProducer::new(Duration::from_millis(100));

  // Complete the example with a consumer that collects timestamps
  let consumer = VecConsumer::<std::time::SystemTime>::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  // Run for a short time to collect a few timestamps
  let ((), consumer) = tokio::time::timeout(Duration::from_millis(250), pipeline.run())
    .await
    .unwrap()
    .unwrap();

  // Verify we got at least one timestamp
  let collected = consumer.into_vec();
  assert!(!collected.is_empty());
}

/// Test: Periodic Processing
///
/// This test recreates the "Periodic Processing" example from README.md.
#[tokio::test]
async fn test_periodic_processing_example() {
  // Example from README.md lines 49-69
  use std::time::Duration;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_timer::IntervalProducer;

  let producer = IntervalProducer::new(Duration::from_millis(50));

  let transformer = MapTransformer::new(|_timestamp: std::time::SystemTime| {
    // Perform periodic task
    "Periodic task executed".to_string()
  });

  let consumer = VecConsumer::<String>::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  // Run for a short time to collect a few messages
  let ((), consumer) = tokio::time::timeout(Duration::from_millis(200), pipeline.run())
    .await
    .unwrap()
    .unwrap();

  // Verify we got messages
  let collected = consumer.into_vec();
  assert!(!collected.is_empty());
  assert!(collected.iter().all(|msg| msg == "Periodic task executed"));
}

/// Test: Error Handling
///
/// This test recreates the "Error Handling" example from README.md.
#[tokio::test]
async fn test_error_handling_example() {
  // Example from README.md lines 141-147
  use std::time::Duration;
  use streamweave_error::ErrorStrategy;
  use streamweave_timer::IntervalProducer;

  let producer =
    IntervalProducer::new(Duration::from_secs(1)).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config.error_strategy(),
    ErrorStrategy::Skip
  ));
}
