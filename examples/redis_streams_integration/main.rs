mod pipeline;

#[allow(unused_imports)]
use pipeline::{consume_from_redis, produce_to_redis, round_trip_example};

#[tokio::main]
#[cfg(feature = "redis")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("produce");

  println!("🚀 StreamWeave Redis Streams Integration Example");
  println!("=================================================");
  println!();

  match example {
    "consume" => {
      println!("Running: Consume from Redis Streams");
      println!("-----------------------------------");
      consume_from_redis().await?;
    }
    "produce" => {
      println!("Running: Produce to Redis Streams");
      println!("---------------------------------");
      produce_to_redis().await?;
    }
    "roundtrip" => {
      println!("Running: Round-trip (Consume → Transform → Produce)");
      println!("---------------------------------------------------");
      round_trip_example().await?;
    }
    _ => {
      println!(
        "Usage: cargo run --example redis_integration --features redis [consume|produce|roundtrip]"
      );
      println!();
      println!("Examples:");
      println!("  cargo run --example redis_integration --features redis produce");
      println!("  cargo run --example redis_integration --features redis consume");
      println!("  cargo run --example redis_integration --features redis roundtrip");
      return Ok(());
    }
  }

  Ok(())
}

#[cfg(not(feature = "redis"))]
fn main() {
  eprintln!("❌ Error: Redis Streams feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'redis' feature to be enabled.");
  eprintln!("Build with: cargo run --example redis_integration --features redis");
  std::process::exit(1);
}
