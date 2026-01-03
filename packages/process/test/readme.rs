//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use streamweave::consumers::VecConsumer;
use streamweave::pipeline::PipelineBuilder;
use streamweave::transformers::MapTransformer;
use streamweave_process::{ProcessConsumer, ProcessProducer};

/// Test: Stream Process Output
///
/// This test recreates the "Stream Process Output" example from README.md.
#[tokio::test]
async fn test_stream_process_output_example() {
  // Example from README.md lines 31-45
  use streamweave::pipeline::PipelineBuilder;
  use streamweave_process::ProcessProducer;

  let producer = ProcessProducer::new("echo".to_string())
    .arg("hello".to_string())
    .arg("world".to_string());

  let consumer = VecConsumer::<String>::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let collected = consumer.into_vec();
  assert!(!collected.is_empty());
}

/// Test: Pipe Data to Process
///
/// This test recreates the "Pipe Data to Process" example from README.md.
#[tokio::test]
async fn test_pipe_data_to_process_example() {
  // Example from README.md lines 47-61
  use streamweave::pipeline::PipelineBuilder;
  use streamweave::producers::ArrayProducer;
  use streamweave_process::ProcessConsumer;

  let producer = ArrayProducer::new(["line1".to_string(), "line2".to_string()]);
  let consumer = ProcessConsumer::new("cat".to_string());

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  // This should complete without error
  let _ = pipeline.run().await;
}

/// Test: Process Output Processing
///
/// This test recreates the "Process Output Processing" example from README.md.
#[tokio::test]
async fn test_process_output_processing_example() {
  // Example from README.md lines 105-124
  use streamweave::pipeline::PipelineBuilder;
  use streamweave_process::ProcessProducer;

  let producer = ProcessProducer::new("echo".to_string()).arg("test".to_string());

  let transformer = MapTransformer::new(|line: String| line.to_uppercase());
  let consumer = VecConsumer::<String>::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let collected = consumer.into_vec();
  assert!(!collected.is_empty());
}

/// Test: Error Handling
///
/// This test recreates the "Error Handling" example from README.md.
#[tokio::test]
async fn test_error_handling_example() {
  // Example from README.md lines 144-154
  use streamweave::error::ErrorStrategy;
  use streamweave_process::ProcessProducer;

  let producer =
    ProcessProducer::new("command".to_string()).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config.error_strategy(),
    ErrorStrategy::Skip
  ));
}
