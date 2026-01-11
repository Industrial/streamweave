//! # Pipeline Test Suite
//!
//! Comprehensive test suite for the pipeline builder and execution system.
//! This module tests the type-safe state machine, error handling strategies,
//! and end-to-end pipeline execution.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **Builder State Machine**: Tests for Empty, HasProducer, HasTransformer, and Complete states
//! - **Pipeline Execution**: End-to-end pipeline runs with various transformations
//! - **Error Handling**: All error strategy variants (Stop, Skip, Retry, Custom)
//! - **Type Safety**: Verification that the type system prevents invalid pipeline construction
//! - **Edge Cases**: Empty inputs, single items, identity transformations, complex chains
//! - **Error Paths**: Internal error handling and panic scenarios
//!
//! ## Test Organization
//!
//! Tests are organized by functionality:
//!
//! - Builder state tests
//! - Transformer chaining tests
//! - Consumer integration tests
//! - Error strategy tests
//! - Edge case tests
//! - Panic path tests (for coverage)

use crate::consumers::VecConsumer;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::pipeline::{Empty, HasProducer, HasTransformer, Pipeline, PipelineBuilder};
use crate::producers::VecProducer;
use crate::transformers::MapTransformer;
use std::sync::Arc;

// ============================================================================
// PipelineBuilder::Empty Tests
// ============================================================================

#[test]
fn test_pipeline_builder_new() {
  let builder = PipelineBuilder::<Empty>::new();
  // Just verify it compiles and can be created
  let _builder = builder;
}

#[test]
fn test_pipeline_builder_default() {
  let builder = PipelineBuilder::<Empty>::default();
  // Just verify it compiles
  let _builder = builder;
}

#[test]
fn test_pipeline_builder_with_error_strategy() {
  let builder = PipelineBuilder::<Empty>::new().with_error_strategy(ErrorStrategy::Skip);

  // Verify error strategy is set (we'll test this when building the pipeline)
  let _builder = builder;
}

// ============================================================================
// PipelineBuilder::HasProducer Tests
// ============================================================================

#[test]
fn test_pipeline_builder_producer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let builder = PipelineBuilder::<Empty>::new().producer(producer);

  // Verify we're in HasProducer state
  let _builder: PipelineBuilder<HasProducer<VecProducer<i32>>> = builder;
}

#[test]
fn test_pipeline_builder_producer_empty_vec() {
  let producer = VecProducer::new(vec![]);
  let builder = PipelineBuilder::<Empty>::new().producer(producer);
  let _builder: PipelineBuilder<HasProducer<VecProducer<i32>>> = builder;
}

// ============================================================================
// PipelineBuilder::HasTransformer Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_builder_transformer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await;

  // Verify we're in HasTransformer state
  let _builder: PipelineBuilder<HasTransformer<VecProducer<i32>, MapTransformer<_, i32, i32>>> =
    builder;
}

#[tokio::test]
async fn test_pipeline_builder_transformer_chain() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let transformer2 = MapTransformer::new(|x: i32| x + 10);

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer1)
    .await
    .transformer(transformer2)
    .await;

  // Verify we're in HasTransformer state with the last transformer
  let _builder: PipelineBuilder<HasTransformer<VecProducer<i32>, MapTransformer<_, i32, i32>>> =
    builder;
}

#[tokio::test]
async fn test_pipeline_builder_transformer_type_change() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| format!("{}", x));

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await;

  // Verify type transformation (i32 -> String)
  let _builder: PipelineBuilder<HasTransformer<VecProducer<i32>, MapTransformer<_, i32, String>>> =
    builder;
}

// ============================================================================
// Pipeline Completion Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_builder_consumer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  // Verify we have a complete Pipeline
  let _pipeline: Pipeline<VecProducer<i32>, MapTransformer<_, i32, i32>, VecConsumer<i32>> =
    pipeline;
}

#[tokio::test]
async fn test_pipeline_builder_consumer_without_transformer() {
  // This should not compile, but let's verify the state machine works
  let producer = VecProducer::new(vec![1, 2, 3]);
  let consumer = VecConsumer::new();

  // This won't compile - need transformer between producer and consumer
  // PipelineBuilder::<Empty>::new()
  //   .producer(producer)
  //   .consumer(consumer);  // Compile error expected

  // Instead, we need to add a transformer (identity transformer)
  let transformer = MapTransformer::new(|x: i32| x);
  let _pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);
}

// ============================================================================
// Pipeline Execution Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_run_simple() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_pipeline_run_empty() {
  let producer = VecProducer::new(vec![]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert!(results.is_empty());
}

#[tokio::test]
async fn test_pipeline_run_chain_transforms() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let transformer2 = MapTransformer::new(|x: i32| x + 1);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer1)
    .await
    .transformer(transformer2)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  // 1*2+1=3, 2*2+1=5, 3*2+1=7
  assert_eq!(results, vec![3, 5, 7]);
}

#[tokio::test]
async fn test_pipeline_run_type_change() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| format!("num:{}", x));
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(
    results,
    vec![
      "num:1".to_string(),
      "num:2".to_string(),
      "num:3".to_string()
    ]
  );
}

#[tokio::test]
async fn test_pipeline_run_large_dataset() {
  let data: Vec<i32> = (1..=1000).collect();
  let producer = VecProducer::new(data);
  let transformer = MapTransformer::new(|x: i32| x * 3);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results.len(), 1000);
  assert_eq!(results[0], 3);
  assert_eq!(results[999], 3000);
}

// ============================================================================
// Pipeline Error Strategy Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_with_error_strategy_stop() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_pipeline_with_error_strategy_skip() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_pipeline_with_error_strategy_retry() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_pipeline_set_error_strategy_on_pipeline() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer)
    .with_error_strategy(ErrorStrategy::Skip);

  let result = pipeline.run().await;
  assert!(result.is_ok());
}

// ============================================================================
// Pipeline Integration Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_full_flow() {
  // Test complete pipeline: producer -> transformer -> consumer
  let producer = VecProducer::new(vec![10, 20, 30]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results, vec![20, 40, 60]);
}

#[tokio::test]
async fn test_pipeline_multiple_transforms() {
  let producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
  let double = MapTransformer::new(|x: i32| x * 2);
  let add_ten = MapTransformer::new(|x: i32| x + 10);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(double)
    .await
    .transformer(add_ten)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  // 1*2+10=12, 2*2+10=14, 3*2+10=16, 4*2+10=18, 5*2+10=20
  assert_eq!(results, vec![12, 14, 16, 18, 20]);
}

#[tokio::test]
async fn test_pipeline_string_transformation() {
  let producer = VecProducer::new(vec!["hello".to_string(), "world".to_string()]);
  let transformer = MapTransformer::new(|s: String| s.to_uppercase());
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results, vec!["HELLO".to_string(), "WORLD".to_string()]);
}

// ============================================================================
// Pipeline Builder State Machine Tests
// ============================================================================

#[test]
fn test_pipeline_builder_state_types() {
  // Verify state machine types prevent incorrect usage
  let producer = VecProducer::new(vec![1, 2, 3]);
  let builder = PipelineBuilder::<Empty>::new().producer(producer);

  // Can't directly add consumer - must add transformer first
  // This is enforced by the type system
  let _builder: PipelineBuilder<HasProducer<VecProducer<i32>>> = builder;
}

#[tokio::test]
async fn test_pipeline_builder_state_progression() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  // Empty state
  let builder1 = PipelineBuilder::<Empty>::new();
  let _builder1: PipelineBuilder<Empty> = builder1;

  // HasProducer state
  let builder2 = PipelineBuilder::<Empty>::new().producer(producer);
  let _builder2: PipelineBuilder<HasProducer<VecProducer<i32>>> = builder2;

  // HasTransformer state
  let producer2 = VecProducer::new(vec![1, 2, 3]);
  let builder3 = PipelineBuilder::<Empty>::new()
    .producer(producer2)
    .transformer(transformer)
    .await;
  let _builder3: PipelineBuilder<HasTransformer<VecProducer<i32>, MapTransformer<_, i32, i32>>> =
    builder3;

  // Complete state (Pipeline)
  let producer3 = VecProducer::new(vec![1, 2, 3]);
  let transformer2 = MapTransformer::new(|x: i32| x * 2);
  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer3)
    .transformer(transformer2)
    .await
    .consumer(consumer);
  let _pipeline: Pipeline<VecProducer<i32>, MapTransformer<_, i32, i32>, VecConsumer<i32>> =
    pipeline;
}

// ============================================================================
// Pipeline Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_error_strategy_inheritance() {
  // Test that error strategy set on builder is used in pipeline
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(5))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_pipeline_single_item() {
  let producer = VecProducer::new(vec![42]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results, vec![84]);
}

#[tokio::test]
async fn test_pipeline_identity_transformer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x); // Identity function
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_pipeline_constant_transformer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|_x: i32| 99); // Always return 99
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  assert_eq!(results, vec![99, 99, 99]);
}

#[tokio::test]
async fn test_pipeline_complex_transformation() {
  let producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
  let transform1 = MapTransformer::new(|x: i32| x * 2);
  let transform2 = MapTransformer::new(|x: i32| x + 1);
  let transform3 = MapTransformer::new(|x: i32| x * 3);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transform1)
    .await
    .transformer(transform2)
    .await
    .transformer(transform3)
    .await
    .consumer(consumer);

  let result = pipeline.run().await;
  assert!(result.is_ok());

  let (_, consumer) = result.unwrap();
  let results = consumer.into_vec();
  // 1*2+1=3*3=9, 2*2+1=5*3=15, 3*2+1=7*3=21, 4*2+1=9*3=27, 5*2+1=11*3=33
  assert_eq!(results, vec![9, 15, 21, 27, 33]);
}

// ============================================================================
// Pipeline Error Handling Internal Method Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_handle_error_stop() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_pipeline_handle_error_skip() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Skip);
}

#[tokio::test]
async fn test_pipeline_handle_error_retry_below_max() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error.retries = 2; // Below max_retries of 3

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Retry);
}

#[tokio::test]
async fn test_pipeline_handle_error_retry_at_max() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error.retries = 3; // At max_retries of 3

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_pipeline_handle_error_retry_above_max() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error.retries = 4; // Above max_retries of 3

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_pipeline_handle_error_custom_handler() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let handler = Arc::new(|error: &StreamError<()>| {
    if error.retries < 2 {
      ErrorAction::Retry
    } else {
      ErrorAction::Skip
    }
  });

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Custom(handler.clone()))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error.retries = 1; // Below threshold of 2

  let action = pipeline._handle_error(error.clone()).await.unwrap();
  assert_eq!(action, ErrorAction::Retry);

  error.retries = 2; // At threshold of 2
  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Skip);
}

// ============================================================================
// Additional Edge Cases and Coverage Tests
// ============================================================================

#[tokio::test]
async fn test_pipeline_with_error_strategy_all_variants_on_builder() {
  // Test all error strategy variants on builder
  let producer1 = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let consumer1 = VecConsumer::new();

  // Test Stop
  let pipeline1 = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(producer1)
    .transformer(transformer1)
    .await
    .consumer(consumer1);
  let _result1 = pipeline1.run().await;

  // Test Skip
  let producer2 = VecProducer::new(vec![1, 2, 3]);
  let transformer2 = MapTransformer::new(|x: i32| x * 2);
  let consumer2 = VecConsumer::new();
  let pipeline2 = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(producer2)
    .transformer(transformer2)
    .await
    .consumer(consumer2);
  let _result2 = pipeline2.run().await;

  // Test Retry
  let producer3 = VecProducer::new(vec![1, 2, 3]);
  let transformer3 = MapTransformer::new(|x: i32| x * 2);
  let consumer3 = VecConsumer::new();
  let pipeline3 = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(5))
    .producer(producer3)
    .transformer(transformer3)
    .await
    .consumer(consumer3);
  let _result3 = pipeline3.run().await;
}

#[tokio::test]
async fn test_pipeline_with_error_strategy_all_variants_on_pipeline() {
  // Test all error strategy variants on Pipeline struct
  // Test Stop
  let producer1 = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let consumer1 = VecConsumer::new();
  let pipeline1 = PipelineBuilder::<Empty>::new()
    .producer(producer1)
    .transformer(transformer1)
    .await
    .consumer(consumer1)
    .with_error_strategy(ErrorStrategy::Stop);
  let _result1 = pipeline1.run().await;

  // Test Skip
  let producer2 = VecProducer::new(vec![1, 2, 3]);
  let transformer2 = MapTransformer::new(|x: i32| x * 2);
  let consumer2 = VecConsumer::new();
  let pipeline2 = PipelineBuilder::<Empty>::new()
    .producer(producer2)
    .transformer(transformer2)
    .await
    .consumer(consumer2)
    .with_error_strategy(ErrorStrategy::Skip);
  let _result2 = pipeline2.run().await;

  // Test Retry
  let producer3 = VecProducer::new(vec![1, 2, 3]);
  let transformer3 = MapTransformer::new(|x: i32| x * 2);
  let consumer3 = VecConsumer::new();
  let pipeline3 = PipelineBuilder::<Empty>::new()
    .producer(producer3)
    .transformer(transformer3)
    .await
    .consumer(consumer3)
    .with_error_strategy(ErrorStrategy::Retry(5));
  let _result3 = pipeline3.run().await;
}

#[tokio::test]
async fn test_pipeline_error_strategy_retry_zero() {
  // Test Retry with 0 max retries
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(0))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error.retries = 0; // At max_retries of 0

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Stop); // Should stop immediately
}

#[tokio::test]
async fn test_pipeline_error_strategy_retry_one() {
  // Test Retry with 1 max retry
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(1))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error.retries = 0; // Below max_retries of 1

  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Retry);

  let mut error2 = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  error2.retries = 1; // At max_retries of 1

  let action2 = pipeline._handle_error(error2).await.unwrap();
  assert_eq!(action2, ErrorAction::Stop);
}

#[tokio::test]
async fn test_pipeline_custom_handler_all_actions() {
  // Test custom handler returning all possible actions
  // Handler that returns Stop
  let handler_stop = Arc::new(|_error: &StreamError<()>| ErrorAction::Stop);
  let producer1 = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let consumer1 = VecConsumer::new();
  let pipeline_stop = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Custom(handler_stop.clone()))
    .producer(producer1)
    .transformer(transformer1)
    .await
    .consumer(consumer1);

  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  let action = pipeline_stop._handle_error(error.clone()).await.unwrap();
  assert_eq!(action, ErrorAction::Stop);

  // Handler that returns Skip
  let handler_skip = Arc::new(|_error: &StreamError<()>| ErrorAction::Skip);
  let producer2 = VecProducer::new(vec![1, 2, 3]);
  let transformer2 = MapTransformer::new(|x: i32| x * 2);
  let consumer2 = VecConsumer::new();
  let pipeline_skip = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Custom(handler_skip.clone()))
    .producer(producer2)
    .transformer(transformer2)
    .await
    .consumer(consumer2);

  let action = pipeline_skip._handle_error(error.clone()).await.unwrap();
  assert_eq!(action, ErrorAction::Skip);

  // Handler that returns Retry
  let handler_retry = Arc::new(|_error: &StreamError<()>| ErrorAction::Retry);
  let producer3 = VecProducer::new(vec![1, 2, 3]);
  let transformer3 = MapTransformer::new(|x: i32| x * 2);
  let consumer3 = VecConsumer::new();
  let pipeline_retry = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Custom(handler_retry.clone()))
    .producer(producer3)
    .transformer(transformer3)
    .await
    .consumer(consumer3);

  let action = pipeline_retry._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Retry);
}

#[tokio::test]
async fn test_pipeline_builder_error_strategy_preserved_through_states() {
  // Test that error strategy is preserved through all builder states
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Retry(10))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  // Verify error strategy is preserved
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Retry);
}

#[tokio::test]
async fn test_pipeline_multiple_error_strategy_changes() {
  // Test changing error strategy multiple times
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .with_error_strategy(ErrorStrategy::Skip)
    .with_error_strategy(ErrorStrategy::Retry(5))
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer)
    .with_error_strategy(ErrorStrategy::Skip);

  // Last strategy should be Skip
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::new("test".to_string(), "TestComponent".to_string()),
  );
  let action = pipeline._handle_error(error).await.unwrap();
  assert_eq!(action, ErrorAction::Skip);
}

// ============================================================================
// Panic Path Tests (for 100% coverage)
// ============================================================================

#[tokio::test]
#[should_panic(expected = "Internal error: producer stream missing")]
async fn test_pipeline_transformer_panic_producer_stream_missing() {
  // Test panic path when producer stream is None in HasProducer state
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    ._test_with_no_producer_stream();

  let _ = builder.transformer(transformer).await;
}

#[tokio::test]
#[should_panic(expected = "Internal error: failed to downcast producer stream")]
async fn test_pipeline_transformer_panic_downcast_fails() {
  // Test panic path when downcast fails (wrong type)
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    ._test_with_wrong_producer_stream_type();

  let _ = builder.transformer(transformer).await;
}

#[tokio::test]
#[should_panic(expected = "Internal error: transformer stream missing")]
async fn test_pipeline_transformer_chain_panic_transformer_stream_missing() {
  // Test panic path when transformer stream is None in HasTransformer state
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let transformer2 = MapTransformer::new(|x: i32| x + 1);

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer1)
    .await
    ._test_with_no_transformer_stream();

  let _ = builder.transformer(transformer2).await;
}

#[tokio::test]
#[should_panic(expected = "Internal error: failed to downcast transformer stream")]
async fn test_pipeline_transformer_chain_panic_downcast_fails() {
  // Test panic path when downcast fails in chained transformer
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let transformer2 = MapTransformer::new(|x: i32| x + 1);

  let builder = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer1)
    .await
    ._test_with_wrong_transformer_stream_type();

  let _ = builder.transformer(transformer2).await;
}

#[test]
#[should_panic(expected = "Internal error: transformer stream missing")]
fn test_pipeline_consumer_panic_transformer_stream_missing() {
  // Test panic path when transformer stream is None in HasTransformer state
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let builder = futures::executor::block_on(async {
    PipelineBuilder::<Empty>::new()
      .producer(producer)
      .transformer(transformer)
      .await
      ._test_with_no_transformer_stream()
  });

  let _ = builder.consumer(consumer);
}

#[test]
#[should_panic(expected = "Internal error: failed to downcast transformer stream")]
fn test_pipeline_consumer_panic_downcast_fails() {
  // Test panic path when downcast fails in consumer
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let builder = futures::executor::block_on(async {
    PipelineBuilder::<Empty>::new()
      .producer(producer)
      .transformer(transformer)
      .await
      ._test_with_wrong_transformer_stream_type()
  });

  let _ = builder.consumer(consumer);
}

#[tokio::test]
#[should_panic(expected = "Internal error: transformer stream missing")]
async fn test_pipeline_run_panic_transformer_stream_missing() {
  // Test panic path when transformer stream is None in Complete state
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer)
    ._test_with_no_transformer_stream();

  pipeline.run().await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "Internal error: consumer missing")]
async fn test_pipeline_run_panic_consumer_missing() {
  // Test panic path when consumer is None in Complete state
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::<Empty>::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer)
    ._test_with_no_consumer();

  pipeline.run().await.unwrap();
}
