//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

/// Test: Produce from Array
///
/// This test recreates the "Produce from Array" example from README.md.
/// The example shows how to create an ArrayProducer and use it in a pipeline.
/// Note: The README example is incomplete (shows `/* process items */`), so we
/// complete it with an identity transformer and VecConsumer to verify it works.
#[tokio::test]
async fn test_produce_from_array() {
  // Example from README.md lines 33-45
  use streamweave_array::ArrayProducer;
  use streamweave_pipeline::PipelineBuilder;

  let array = [1, 2, 3, 4, 5];
  let producer = ArrayProducer::new(array);

  // Complete the example: README shows `/* process items */` - we use identity transformer
  // and VecConsumer to verify the producer works
  let transformer = MapTransformer::new(|x: i32| x); // Identity transformation
  let consumer = VecConsumer::<i32>::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();

  // Verify the producer emitted all items
  let collected = consumer.into_vec();
  assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}

/// Test: Consume to Array
///
/// This test recreates the "Consume to Array" example from README.md.
/// The example shows how to create an ArrayConsumer and use it in a pipeline.
/// Note: The README example is incomplete (shows `/* produce items */`), so we
/// complete it with an ArrayProducer and identity transformer to verify it works.
#[tokio::test]
async fn test_consume_to_array() {
  // Example from README.md lines 49-62
  use streamweave_array::ArrayConsumer;
  use streamweave_pipeline::PipelineBuilder;

  let consumer = ArrayConsumer::<i32, 5>::new();

  // Complete the example: README shows `/* produce items */` - we use ArrayProducer
  // and identity transformer to verify the consumer works
  let array = [1, 2, 3, 4, 5];
  let producer = ArrayProducer::new(array);
  let transformer = MapTransformer::new(|x: i32| x); // Identity transformation

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();

  let array_result = consumer.into_array();

  // Verify all items were consumed
  let collected: Vec<i32> = array_result
    .iter()
    .filter_map(|opt| opt.as_ref().copied())
    .collect();
  assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}

/// Test: Array Transformation Pipeline
///
/// This test recreates the "Array Transformation Pipeline" example from README.md.
/// The example shows how to transform array elements using a transformer.
#[tokio::test]
async fn test_array_transformation_pipeline() {
  // Example from README.md lines 108-122
  use streamweave_array::{ArrayConsumer, ArrayProducer};
  use streamweave_pipeline::PipelineBuilder;

  let input = [1, 2, 3, 4, 5];
  let producer = ArrayProducer::new(input);
  let consumer = ArrayConsumer::<i32, 5>::new();

  // Use MapTransformer to double each element (as shown in README comment)
  let transformer = MapTransformer::new(|x: i32| x * 2);

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();

  let array_result = consumer.into_array();

  // Verify transformation: each element should be doubled
  let collected: Vec<i32> = array_result
    .iter()
    .filter_map(|opt| opt.as_ref().copied())
    .collect();
  assert_eq!(collected, vec![2, 4, 6, 8, 10]);
}

/// Test: Error Handling Configuration
///
/// This test recreates the "Error Handling" example from README.md.
/// The example shows how to configure error handling strategies.
#[tokio::test]
async fn test_error_handling_configuration() {
  // Example from README.md lines 128-136
  use streamweave_array::ArrayProducer;
  use streamweave_error::ErrorStrategy;

  let array = [1, 2, 3];
  let producer = ArrayProducer::new(array)
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("my_array_producer".to_string());

  // Verify the configuration was set correctly
  let config = producer.config();
  assert_eq!(config.name, Some("my_array_producer".to_string()));

  // Verify the producer still works
  let transformer = MapTransformer::new(|x: i32| x); // Identity transformation
  let consumer = VecConsumer::<i32>::new();
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let collected = consumer.into_vec();
  assert_eq!(collected, vec![1, 2, 3]);
}

/// Test: Error Handling with ErrorStrategy
///
/// This test recreates the error handling example from README.md lines 168-173.
#[tokio::test]
async fn test_error_handling_with_strategy() {
  // Example from README.md lines 168-173
  use streamweave_array::ArrayProducer;
  use streamweave_error::ErrorStrategy;

  let array = [1, 2, 3, 4, 5];
  let producer = ArrayProducer::new(array).with_error_strategy(ErrorStrategy::Skip);

  // Verify the configuration was set
  let config = producer.config();
  match &config.error_strategy {
    ErrorStrategy::Skip => {
      // Expected
    }
    _ => panic!("Expected ErrorStrategy::Skip"),
  }

  // Verify the producer works
  let transformer = MapTransformer::new(|x: i32| x); // Identity transformation
  let consumer = VecConsumer::<i32>::new();
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let collected = consumer.into_vec();
  assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}
