//! Integration tests for README examples
//!
//! These tests verify that all code examples in the README compile and run correctly.

use futures::StreamExt;
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;
use streamweave_vec::{VecConsumer, VecProducer};

#[tokio::test]
async fn test_readme_produce_from_vector() {
  // Example: Produce from Vector
  let data = vec![1, 2, 3, 4, 5];
  let producer = VecProducer::new(data.clone());
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let result = consumer.into_vec();
  assert_eq!(result, data);
}

#[tokio::test]
async fn test_readme_consume_to_vector() {
  // Example: Consume to Vector
  let producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
  let consumer = VecConsumer::<i32>::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let vec = consumer.into_vec();
  assert_eq!(vec, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_readme_vector_transformation_pipeline() {
  // Example: Vector Transformation Pipeline
  let input = vec![1, 2, 3, 4, 5];
  let producer = VecProducer::new(input.clone());
  let consumer = VecConsumer::<i32>::with_capacity(10);

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(MapTransformer::new(|x: i32| x * 2))
    .await
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();
  let result = consumer.into_vec();
  let expected: Vec<i32> = input.iter().map(|x| x * 2).collect();
  assert_eq!(result, expected);
}

#[tokio::test]
async fn test_readme_pre_allocated_capacity() {
  // Example: Pre-allocated Capacity
  let consumer = VecConsumer::<i32>::with_capacity(1000);
  assert_eq!(consumer.vec.capacity(), 1000);
}

#[tokio::test]
async fn test_readme_error_handling() {
  // Example: Error Handling
  use streamweave_error::ErrorStrategy;

  let producer = VecProducer::new(vec![1, 2, 3]).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}
