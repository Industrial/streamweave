mod pipeline;

use pipeline::{consume_from_kafka, produce_to_kafka, round_trip_example};

#[tokio::main]
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("produce");

  println!("🚀 StreamWeave Kafka Integration Example");
  println!("==========================================");
  println!();

  match example {
    "consume" => {
      println!("Running: Consume from Kafka");
      println!("---------------------------");
      consume_from_kafka().await?;
    }
    "produce" => {
      println!("Running: Produce to Kafka");
      println!("-------------------------");
      produce_to_kafka().await?;
    }
    "roundtrip" => {
      println!("Running: Round-trip (Consume → Transform → Produce)");
      println!("---------------------------------------------------");
      round_trip_example().await?;
    }
    _ => {
      println!("Usage: cargo run --example kafka_integration --features kafka [consume|produce|roundtrip]");
      println!();
      println!("Examples:");
      println!("  cargo run --example kafka_integration --features kafka produce");
      println!("  cargo run --example kafka_integration --features kafka consume");
      println!("  cargo run --example kafka_integration --features kafka roundtrip");
      return Ok(());
    }
  }

  Ok(())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "kafka")))]
fn main() {
  eprintln!("❌ Error: Kafka feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'kafka' feature to be enabled.");
  eprintln!("Build with: cargo run --example kafka_integration --features kafka");
  std::process::exit(1);
}

