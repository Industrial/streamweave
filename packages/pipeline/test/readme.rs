//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use streamweave_array::ArrayProducer;
use streamweave_error::ErrorStrategy;
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

/// Test: Simple Pipeline
///
/// This test recreates the "Simple Pipeline" example from README.md lines 33-48.
#[tokio::test]
async fn test_simple_pipeline() {
  use streamweave_array::ArrayProducer;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
    .transformer(MapTransformer::new(|x| x * 2))
    .await
    .consumer(VecConsumer::new());

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6, 8, 10]);
}

/// Test: Basic Pipeline
///
/// This test recreates the "Basic Pipeline" example from README.md lines 89-104.
#[tokio::test]
async fn test_basic_pipeline() {
  use streamweave_array::ArrayProducer;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3]))
    .transformer(MapTransformer::new(|x| x * 2))
    .await
    .consumer(VecConsumer::new());

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6]);
}

/// Test: Multiple Transformers
///
/// This test recreates the "Multiple Transformers" example from README.md lines 106-119.
#[tokio::test]
async fn test_multiple_transformers() {
  use streamweave_array::ArrayProducer;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
    .transformer(MapTransformer::new(|x| x * 2)) // First transformer
    .await
    .transformer(MapTransformer::new(|x| x + 1)) // Second transformer
    .await
    .consumer(VecConsumer::new());

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  // First transformer: [1,2,3,4,5] -> [2,4,6,8,10]
  // Second transformer: [2,4,6,8,10] -> [3,5,7,9,11]
  assert_eq!(results, vec![3, 5, 7, 9, 11]);
}

/// Test: Error Handling Strategies - Stop
///
/// This test recreates the "Error Handling Strategies" example from README.md lines 121-148.
#[tokio::test]
async fn test_error_handling_stop() {
  use streamweave_array::ArrayProducer;
  use streamweave_error::ErrorStrategy;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  // Stop on first error (default)
  let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(ArrayProducer::new(vec![1, 2, 3]))
    .transformer(MapTransformer::new(|x| x * 2))
    .await
    .consumer(VecConsumer::new());

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6]);
}

/// Test: Error Handling Strategies - Skip
///
/// This test recreates the "Error Handling Strategies" example from README.md lines 135-140.
#[tokio::test]
async fn test_error_handling_skip() {
  use streamweave_array::ArrayProducer;
  use streamweave_error::ErrorStrategy;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  // Skip errors and continue
  let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(ArrayProducer::new(vec![1, 2, 3]))
    .transformer(MapTransformer::new(|x| x * 2))
    .await
    .consumer(VecConsumer::new());

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6]);
}

/// Test: Error Handling Strategies - Retry
///
/// This test recreates the "Error Handling Strategies" example from README.md lines 142-147.
#[tokio::test]
async fn test_error_handling_retry() {
  use streamweave_array::ArrayProducer;
  use streamweave_error::ErrorStrategy;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  // Retry up to 3 times
  let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .producer(ArrayProducer::new(vec![1, 2, 3]))
    .transformer(MapTransformer::new(|x| x * 2))
    .await
    .consumer(VecConsumer::new());

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6]);
}

/// Test: Pipeline Configuration
///
/// This test recreates the "Pipeline Configuration" example from README.md lines 150-162.
#[tokio::test]
async fn test_pipeline_configuration() {
  use streamweave_array::ArrayProducer;
  use streamweave_error::ErrorStrategy;
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  let producer = ArrayProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer)
    .with_error_strategy(ErrorStrategy::Skip);

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6]);
}

/// Test: Error Handling in Pipelines
///
/// This test recreates the "Error Handling in Pipelines" example from README.md lines 188-211.
#[tokio::test]
async fn test_error_handling_in_pipelines() {
  use streamweave_array::ArrayProducer;
  use streamweave_error::{ErrorStrategy, PipelineError};
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  let producer = ArrayProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  match pipeline.run().await {
    Ok((_, consumer)) => {
      // Pipeline completed successfully
      let results = consumer.into_inner();
      assert_eq!(results, vec![2, 4, 6]);
    }
    Err(PipelineError { .. }) => {
      // Error occurred, pipeline stopped
      panic!("Pipeline execution failed unexpectedly");
    }
  }
}

/// Test: Custom Error Handling
///
/// This test recreates the "Custom Error Handling" example from README.md lines 213-233.
#[tokio::test]
async fn test_custom_error_handling() {
  use streamweave_array::ArrayProducer;
  use streamweave_error::{ErrorAction, ErrorStrategy, StreamError};
  use streamweave_pipeline::PipelineBuilder;
  use streamweave_transformers::MapTransformer;
  use streamweave_vec::VecConsumer;

  let producer = ArrayProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x| x * 2);
  let consumer = VecConsumer::new();

  let custom_strategy = ErrorStrategy::new_custom(|error: &StreamError<()>| {
    if error.retries < 3 {
      ErrorAction::Retry
    } else {
      ErrorAction::Skip
    }
  });

  let pipeline = PipelineBuilder::new()
    .with_error_strategy(custom_strategy)
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let (_, consumer) = pipeline.run().await.unwrap();
  let results = consumer.into_inner();
  assert_eq!(results, vec![2, 4, 6]);
}
