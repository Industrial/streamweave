mod pipeline;

use pipeline::{
  stop_strategy_example, skip_strategy_example, retry_strategy_example, custom_strategy_example,
  component_level_example,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("stop");

  println!("🚀 StreamWeave Error Handling Example");
  println!("=====================================");
  println!();

  match example {
    "stop" => {
      println!("Running: Stop Strategy Example");
      println!("------------------------------");
      stop_strategy_example().await?;
    }
    "skip" => {
      println!("Running: Skip Strategy Example");
      println!("------------------------------");
      skip_strategy_example().await?;
    }
    "retry" => {
      println!("Running: Retry Strategy Example");
      println!("-------------------------------");
      retry_strategy_example().await?;
    }
    "custom" => {
      println!("Running: Custom Strategy Example");
      println!("--------------------------------");
      custom_strategy_example().await?;
    }
    "component" => {
      println!("Running: Component-Level Error Handling");
      println!("---------------------------------------");
      component_level_example().await?;
    }
    _ => {
      println!(
        "Usage: cargo run --example error_handling [stop|skip|retry|custom|component]"
      );
      println!();
      println!("Examples:");
      println!("  cargo run --example error_handling stop");
      println!("  cargo run --example error_handling skip");
      println!("  cargo run --example error_handling retry");
      println!("  cargo run --example error_handling custom");
      println!("  cargo run --example error_handling component");
      return Ok(());
    }
  }

  Ok(())
}

