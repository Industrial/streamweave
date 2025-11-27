use std::time::Duration;
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformers::batch::batch_transformer::BatchTransformer,
  transformers::circuit_breaker::circuit_breaker_transformer::CircuitBreakerTransformer,
  transformers::rate_limit::rate_limit_transformer::RateLimitTransformer,
  transformers::retry::retry_transformer::RetryTransformer,
};

pub async fn circuit_breaker_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔌 Circuit Breaker Example");
  println!("--------------------------");
  println!("Demonstrates protecting external service calls from cascading failures");

  let data: Vec<i32> = (1..=10).collect();
  let producer = VecProducer::new(data);

  let circuit_breaker = CircuitBreakerTransformer::new(
    3,                      // Failure threshold
    Duration::from_secs(5), // Reset timeout
  )
  .with_name("circuit-breaker".to_string());

  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(circuit_breaker)
    .consumer(consumer);
    

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();
  println!("✅ Processed {} items", results.len());
  println!("💡 Circuit breaker prevents cascading failures");
  Ok(())
}

pub async fn retry_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔄 Retry Transformer Example");
  println!("----------------------------");
  println!("Demonstrates automatic retries with backoff");

  let data: Vec<i32> = (1..=5).collect();
  let producer = VecProducer::new(data);

  let retry = RetryTransformer::new(
    3,                          // Max retries
    Duration::from_millis(100), // Backoff
  )
  .with_name("retry".to_string());

  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(retry)
    .consumer(consumer);
    

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();
  println!("✅ Processed {} items with retry logic", results.len());
  println!("💡 Retries help with transient failures");
  Ok(())
}

pub async fn batch_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📦 Batch Transformer Example");
  println!("----------------------------");
  println!("Demonstrates grouping items into batches");

  let data: Vec<i32> = (1..=10).collect();
  let producer = VecProducer::new(data);

  let batch = BatchTransformer::new(3)? // Batch size 3
    .with_name("batch".to_string());

  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(batch)
    .consumer(consumer);
    

  let (_, result_consumer) = pipeline.run().await?;
  let results: Vec<Vec<i32>> = result_consumer.into_vec();
  println!("✅ Created {} batches", results.len());
  for (i, batch) in results.iter().enumerate() {
    println!("  Batch {}: {:?}", i + 1, batch);
  }
  println!("💡 Batching reduces per-item overhead");
  Ok(())
}

pub async fn rate_limit_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("⏱️  Rate Limit Transformer Example");
  println!("----------------------------------");
  println!("Demonstrates throughput control");

  let data: Vec<i32> = (1..=10).collect();
  let producer = VecProducer::new(data);

  let rate_limit = RateLimitTransformer::new(
    3,                      // 3 items per window
    Duration::from_secs(1), // 1 second window
  )
  .with_name("rate-limit".to_string());

  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(rate_limit)
    .consumer(consumer);
    

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();
  println!("✅ Processed {} items (rate limited)", results.len());
  println!("💡 Rate limiting prevents overwhelming downstream systems");
  Ok(())
}
