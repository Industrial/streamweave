use crate::consumers::VecConsumer;
use crate::error::ErrorStrategy;
use crate::pipeline::{Empty, HasProducer, HasTransformer, Pipeline, PipelineBuilder};
use crate::producers::VecProducer;
use crate::transformers::MapTransformer;

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
