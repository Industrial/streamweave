mod pipeline;

use pipeline::run_advanced_pipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🚀 StreamWeave Advanced Pipeline Example");
  println!("=========================================");
  println!("This example demonstrates an advanced data processing pipeline with:");
  println!("1. FileProducer: Reads data from a file");
  println!("2. MapTransformer: Converts strings to integers");
  println!("3. BatchTransformer: Groups items in batches of 3");
  println!("4. MapTransformer: Converts batches to formatted strings");
  println!("5. RateLimitTransformer: Limits processing to 1 item per second");
  println!("6. CircuitBreakerTransformer: Handles failures gracefully");
  println!("7. RetryTransformer: Retries failed operations up to 3 times");
  println!("8. FileConsumer: Writes results to another file");
  println!();

  run_advanced_pipeline().await?;

  println!();
  println!("✅ Advanced pipeline example completed successfully!");
  println!("Key Features Demonstrated:");
  println!("• File I/O: Reading from and writing to files");
  println!("• Data Transformation: String to integer conversion");
  println!("• Batching: Grouping data for efficient processing");
  println!("• Rate Limiting: Controlling processing speed");
  println!("• Circuit Breaker: Fault tolerance and failure handling");
  println!("• Retry Logic: Automatic retry of failed operations");
  println!("• Error Handling: Graceful handling of invalid data");

  Ok(())
}
