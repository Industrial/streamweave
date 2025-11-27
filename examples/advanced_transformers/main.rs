mod pipeline;

use pipeline::{batch_example, circuit_breaker_example, rate_limit_example, retry_example};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("circuit-breaker");

  println!("🚀 StreamWeave Advanced Transformers Example");
  println!("=============================================");
  println!();

  match example {
    "circuit-breaker" => circuit_breaker_example().await?,
    "retry" => retry_example().await?,
    "batch" => batch_example().await?,
    "rate-limit" => rate_limit_example().await?,
    _ => {
      println!(
        "Usage: cargo run --example advanced_transformers [circuit-breaker|retry|batch|rate-limit]"
      );
      return Ok(());
    }
  }

  Ok(())
}
