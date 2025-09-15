use std::time::Duration;
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  pipeline::PipelineBuilder,
  producers::file::file_producer::FileProducer,
  transformers::{
    batch::batch_transformer::BatchTransformer,
    circuit_breaker::circuit_breaker_transformer::CircuitBreakerTransformer,
    map::map_transformer::MapTransformer, 
    rate_limit::rate_limit_transformer::RateLimitTransformer,
    retry::retry_transformer::RetryTransformer,
  },
  transformer::{Transformer, TransformerConfig},
};
use tokio::fs;

#[tokio::test]
async fn test_advanced_pipeline_processes_data_correctly() {
  // Create a pipeline that processes data through multiple transformers
  let mut map_transformer1 = MapTransformer::new(|s: String| {
    s.parse::<i32>().unwrap_or_default()
  });
  map_transformer1.set_config(TransformerConfig::default().with_name("string_to_int".to_string()));

  let batch_transformer = BatchTransformer::new(3).unwrap();

  let mut map_transformer2 = MapTransformer::new(|batch: Vec<i32>| {
    batch
      .iter()
      .map(|i| i.to_string())
      .collect::<Vec<String>>()
      .join(", ")
  });
  map_transformer2.set_config(TransformerConfig::default().with_name("batch_to_string".to_string()));

  let rate_limit_transformer = RateLimitTransformer::new(1, Duration::from_millis(10));
  let circuit_breaker_transformer = CircuitBreakerTransformer::new(3, Duration::from_secs(1));
  let retry_transformer = RetryTransformer::new(3, Duration::from_millis(10));

  let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("test_input.txt".to_string()))
    .transformer(map_transformer1)
    .transformer(batch_transformer)
    .transformer(map_transformer2)
    .transformer(rate_limit_transformer)
    .transformer(circuit_breaker_transformer)
    .transformer(retry_transformer)
    ._consumer(VecConsumer::<String>::new());

  // Create test input file
  fs::write("test_input.txt", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n").await.unwrap();

  // Run the pipeline
  let result = pipeline.run().await;
  
  // Clean up test file
  let _ = fs::remove_file("test_input.txt").await;

  // Verify the pipeline completed successfully
  assert!(result.is_ok());
  
  let (_, consumer) = result.unwrap();
  let output = consumer.into_vec();
  
  // Should have processed the data in batches
  assert!(!output.is_empty());
  
  // Each batch should contain 3 items (except possibly the last one)
  for batch in &output {
    let items: Vec<&str> = batch.split(", ").collect();
    assert!(items.len() <= 3);
    assert!(items.len() >= 1);
  }
}

#[tokio::test]
async fn test_advanced_pipeline_with_different_batch_size() {
  // Test with a different batch size
  let mut map_transformer1 = MapTransformer::new(|s: String| {
    s.parse::<i32>().unwrap_or_default()
  });
  map_transformer1.set_config(TransformerConfig::default().with_name("string_to_int".to_string()));

  let batch_transformer = BatchTransformer::new(2).unwrap();

  let mut map_transformer2 = MapTransformer::new(|batch: Vec<i32>| {
    batch
      .iter()
      .map(|i| i.to_string())
      .collect::<Vec<String>>()
      .join(", ")
  });
  map_transformer2.set_config(TransformerConfig::default().with_name("batch_to_string".to_string()));

  let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("test_input2.txt".to_string()))
    .transformer(map_transformer1)
    .transformer(batch_transformer)
    .transformer(map_transformer2)
    ._consumer(VecConsumer::<String>::new());

  // Create test input file
  fs::write("test_input2.txt", "1\n2\n3\n4\n5\n").await.unwrap();

  // Run the pipeline
  let result = pipeline.run().await;
  
  // Clean up test file
  let _ = fs::remove_file("test_input2.txt").await;

  // Verify the pipeline completed successfully
  assert!(result.is_ok());
  
  let (_, consumer) = result.unwrap();
  let output = consumer.into_vec();
  
  // Should have processed the data in batches of 2
  assert!(!output.is_empty());
  
  // Each batch should contain 2 items (except possibly the last one)
  for batch in &output {
    let items: Vec<&str> = batch.split(", ").collect();
    assert!(items.len() <= 2);
    assert!(items.len() >= 1);
  }
}

#[tokio::test]
async fn test_advanced_pipeline_handles_invalid_data() {
  // Test that the pipeline handles invalid data gracefully
  let mut map_transformer1 = MapTransformer::new(|s: String| {
    s.parse::<i32>().unwrap_or_default()
  });
  map_transformer1.set_config(TransformerConfig::default().with_name("string_to_int".to_string()));

  let batch_transformer = BatchTransformer::new(2).unwrap();

  let mut map_transformer2 = MapTransformer::new(|batch: Vec<i32>| {
    batch
      .iter()
      .map(|i| i.to_string())
      .collect::<Vec<String>>()
      .join(", ")
  });
  map_transformer2.set_config(TransformerConfig::default().with_name("batch_to_string".to_string()));

  let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("test_input3.txt".to_string()))
    .transformer(map_transformer1)
    .transformer(batch_transformer)
    .transformer(map_transformer2)
    ._consumer(VecConsumer::<String>::new());

  // Create test input file with invalid data
  fs::write("test_input3.txt", "1\ninvalid\n3\n4\n5\n").await.unwrap();

  // Run the pipeline
  let result = pipeline.run().await;
  
  // Clean up test file
  let _ = fs::remove_file("test_input3.txt").await;

  // Verify the pipeline completed successfully (invalid data should be converted to 0)
  assert!(result.is_ok());
  
  let (_, consumer) = result.unwrap();
  let output = consumer.into_vec();
  
  // Should have processed the data
  assert!(!output.is_empty());
  
  // Should contain the processed data including the invalid entry converted to 0
  let all_data: String = output.join(" ");
  assert!(all_data.contains("0")); // The invalid entry should be converted to 0
}
