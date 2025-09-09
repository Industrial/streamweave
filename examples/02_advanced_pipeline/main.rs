use std::time::Duration;
use streamweave::{
  consumers::file::file_consumer::FileConsumer,
  error::ErrorStrategy,
  pipeline::PipelineBuilder,
  producers::file::file_producer::FileProducer,
  transformers::{
    batch::batch_transformer::BatchTransformer,
    circuit_breaker::circuit_breaker_transformer::CircuitBreakerTransformer,
    map::map_transformer::MapTransformer, rate_limit::rate_limit_transformer::RateLimitTransformer,
    retry::retry_transformer::RetryTransformer,
  },
};
use tempfile::NamedTempFile;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create temporary files for input and output
  let input_file = NamedTempFile::new()?;
  let output_file = NamedTempFile::new()?;

  // Write some test data to the input file
  fs::write(input_file.path(), "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n").await?;

  // Create a pipeline that:
  // 1. Reads from a file
  // 2. Maps strings to integers
  // 3. Batches items in groups of 3
  // 4. Converts batches to strings
  // 5. Rate limits to 1 item per second
  // 6. Uses circuit breaker to handle failures
  // 7. Retries failed operations up to 3 times
  // 8. Writes results to another file
  let _pipeline = PipelineBuilder::new()
    .producer(FileProducer::new(
      input_file.path().to_str().unwrap().to_string(),
    ))
    .transformer(MapTransformer::new(|s: String| {
      s.parse::<i32>().unwrap_or_default()
    }))
    .transformer(BatchTransformer::new(3).unwrap())
    .transformer(MapTransformer::new(|batch: Vec<i32>| {
      batch
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<String>>()
        .join(", ")
    }))
    .transformer(RateLimitTransformer::new(1, Duration::from_secs(1)))
    .transformer(CircuitBreakerTransformer::new(3, Duration::from_secs(5)))
    .transformer(RetryTransformer::new(3, Duration::from_millis(100)))
    ._consumer(
      FileConsumer::new(output_file.path().to_str().unwrap().to_string())
        .with_error_strategy(ErrorStrategy::<String>::Stop),
    )
    .run()
    .await?;

  // Read and verify the output
  let output = fs::read_to_string(output_file.path()).await?;
  println!("Pipeline output:\n{}", output);

  Ok(())
}
