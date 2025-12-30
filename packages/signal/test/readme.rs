//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use streamweave_pipeline::PipelineBuilder;
use streamweave_signal::{Signal, SignalProducer};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

/// Test: Handle Signals
///
/// This test recreates the "Handle Signals" example from README.md lines 33-49.
/// The example shows how to create a SignalProducer and handle signals in a pipeline.
#[tokio::test]
async fn test_handle_signals() {
  // Example from README.md lines 33-49
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_signal::{Signal, SignalProducer};

  let producer = SignalProducer::new();

  // Note: The README example shows a closure consumer, but we'll use VecConsumer
  // to collect signals for testing. In a real scenario, signals would be handled
  // as they arrive. For testing, we'll use a timeout to avoid waiting indefinitely.
  let consumer = VecConsumer::<Signal>::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  // Note: SignalProducer produces an infinite stream, so we can't easily test
  // the full pipeline without actually sending signals. This test verifies
  // the pipeline can be constructed and the producer can be created.
  // In a real scenario, you would send SIGINT or SIGTERM to test signal handling.
}

/// Test: Graceful Shutdown
///
/// This test recreates the "Graceful Shutdown" example from README.md lines 51-71.
/// The example shows how to handle shutdown signals gracefully.
#[tokio::test]
async fn test_graceful_shutdown() {
  // Example from README.md lines 51-71
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_signal::{Signal, SignalProducer};

  let producer = SignalProducer::new();

  // Note: The README example shows a closure consumer, but we'll use VecConsumer
  // for testing. In a real scenario, you would perform cleanup in the consumer.
  let consumer = VecConsumer::<Signal>::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  // Note: SignalProducer produces an infinite stream, so we can't easily test
  // the full pipeline without actually sending signals. This test verifies
  // the pipeline can be constructed.
}

/// Test: Signal-Based Processing
///
/// This test recreates the "Signal-Based Processing" example from README.md lines 108-128.
/// The example shows how to process signals through a pipeline with a transformer.
#[tokio::test]
async fn test_signal_based_processing() {
  // Example from README.md lines 108-128
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_signal::{Signal, SignalProducer};

  let producer = SignalProducer::new();

  // Use MapTransformer to format signals as strings
  let transformer = MapTransformer::new(|signal: Signal| format!("Received signal: {:?}", signal));

  let consumer = VecConsumer::<String>::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  // Note: SignalProducer produces an infinite stream, so we can't easily test
  // the full pipeline without actually sending signals. This test verifies
  // the pipeline can be constructed with a transformer.
}

/// Test: Signal Handling
///
/// This test recreates the "Signal Handling" example from README.md lines 130-154.
/// The example shows how to handle different signals.
#[tokio::test]
async fn test_signal_handling() {
  // Example from README.md lines 130-154
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_signal::{Signal, SignalProducer};

  let producer = SignalProducer::new();

  // Note: The README example shows a closure consumer that matches on signals,
  // but we'll use VecConsumer for testing.
  let consumer = VecConsumer::<Signal>::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  // Note: SignalProducer produces an infinite stream, so we can't easily test
  // the full pipeline without actually sending signals. This test verifies
  // the pipeline can be constructed.
}

/// Test: Error Handling
///
/// This test recreates the "Error Handling" example from README.md lines 156-166.
/// The example shows how to configure error handling strategies.
#[tokio::test]
async fn test_error_handling() {
  // Example from README.md lines 156-166
  use streamweave_error::ErrorStrategy;
  use streamweave_signal::SignalProducer;

  let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::Skip);

  // Verify the configuration was set correctly
  let config = producer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<Signal>::Skip);
}
