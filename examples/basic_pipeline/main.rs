mod pipeline;

use pipeline::run_basic_pipeline;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🚀 StreamWeave Basic Pipeline Example");
  println!("=====================================");
  println!("This example demonstrates a simple data processing pipeline:");
  println!("1. Produces numbers from 1 to 5");
  println!("2. Doubles each number");
  println!("3. Prints the result to the console");
  println!();

  run_basic_pipeline().await?;

  println!();
  println!("✅ Basic pipeline example completed successfully!");
  println!("Key Features Demonstrated:");
  println!("• RangeProducer: Generates sequential numbers");
  println!("• MapTransformer: Transforms data (doubling in this case)");
  println!("• ConsoleConsumer: Outputs results to console");
  println!("• PipelineBuilder: Orchestrates the data flow");

  Ok(())
}
