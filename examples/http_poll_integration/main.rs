mod pipeline;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-poll"))]
use pipeline::{basic_polling, delta_detection_example, pagination_example, rate_limited_polling};

#[tokio::main]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-poll"))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("basic");

  println!("🚀 StreamWeave HTTP Polling Integration Example");
  println!("==============================================");
  println!();

  match example {
    "basic" => {
      println!("Running: Basic HTTP Polling");
      println!("---------------------------");
      basic_polling().await?;
    }
    "pagination" => {
      println!("Running: Pagination Handling");
      println!("----------------------------");
      pagination_example().await?;
    }
    "delta" => {
      println!("Running: Delta Detection");
      println!("------------------------");
      delta_detection_example().await?;
    }
    "rate-limit" => {
      println!("Running: Rate Limited Polling");
      println!("-----------------------------");
      rate_limited_polling().await?;
    }
    _ => {
      println!(
        "Usage: cargo run --example http_poll_integration --features http-poll [basic|pagination|delta|rate-limit]"
      );
      println!();
      println!("Examples:");
      println!("  cargo run --example http_poll_integration --features http-poll basic");
      println!("  cargo run --example http_poll_integration --features http-poll pagination");
      println!("  cargo run --example http_poll_integration --features http-poll delta");
      println!("  cargo run --example http_poll_integration --features http-poll rate-limit");
      return Ok(());
    }
  }

  Ok(())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "http-poll")))]
fn main() {
  eprintln!("❌ Error: HTTP polling feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'http-poll' feature to be enabled.");
  eprintln!("Build with: cargo run --example http_poll_integration --features http-poll");
  std::process::exit(1);
}
