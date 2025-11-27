mod pipeline;

use pipeline::{checkpoint_example, deduplication_example};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("deduplication");

  println!("🚀 StreamWeave Exactly-Once Processing Example");
  println!("==============================================");
  println!();

  match example {
    "deduplication" => deduplication_example().await?,
    "checkpoint" => checkpoint_example().await?,
    _ => {
      println!("Usage: cargo run --example exactly_once [deduplication|checkpoint]");
      return Ok(());
    }
  }

  Ok(())
}
