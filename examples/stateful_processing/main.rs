mod pipeline;

use pipeline::{moving_average_example, running_sum_example, state_checkpoint_example};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("running-sum");

  println!("🚀 StreamWeave Stateful Processing Example");
  println!("===========================================");
  println!();

  match example {
    "running-sum" => {
      println!("Running: Running Sum Example");
      println!("---------------------------");
      running_sum_example().await?;
    }
    "moving-average" => {
      println!("Running: Moving Average Example");
      println!("-------------------------------");
      moving_average_example().await?;
    }
    "checkpoint" => {
      println!("Running: State Checkpoint Example");
      println!("--------------------------------");
      state_checkpoint_example().await?;
    }
    _ => {
      println!(
        "Usage: cargo run --example stateful_processing [running-sum|moving-average|checkpoint]"
      );
      println!();
      println!("Examples:");
      println!("  cargo run --example stateful_processing running-sum");
      println!("  cargo run --example stateful_processing moving-average");
      println!("  cargo run --example stateful_processing checkpoint");
      return Ok(());
    }
  }

  Ok(())
}
