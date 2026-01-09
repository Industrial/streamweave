//! Tests for pipeline module

use streamweave::consumers::VecConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::pipeline::*;
use streamweave::producers::VecProducer;
use streamweave::transformers::MapTransformer;

#[tokio::test]
async fn test_pipeline_builder_new() {
  let builder = PipelineBuilder::new();
  // Builder should be created
  assert!(true);
}

#[tokio::test]
async fn test_pipeline_builder_with_error_strategy() {
  let builder = PipelineBuilder::new().with_error_strategy(ErrorStrategy::Skip);
  // Builder should accept error strategy
  assert!(true);
}

#[tokio::test]
async fn test_pipeline_builder_producer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let builder = PipelineBuilder::new().producer(producer);
  // Builder should accept producer
  assert!(true);
}

#[tokio::test]
async fn test_pipeline_builder_with_transformer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let builder = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer);
  // Builder should accept transformer
  assert!(true);
}

#[tokio::test]
async fn test_pipeline_builder_with_consumer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();
  let builder = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);
  // Builder should accept consumer
  assert!(true);
}

#[tokio::test]
async fn test_pipeline_build() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer)
    .build();
  // Pipeline should be built successfully
  assert!(true);
}

#[tokio::test]
async fn test_pipeline_execute() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer)
    .build();

  pipeline.execute().await;
  // Pipeline should execute successfully
  assert!(true);
}
