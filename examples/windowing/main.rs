mod pipeline;

use pipeline::{count_window_example, sliding_window_example, tumbling_window_example};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("tumbling");

  println!("🚀 StreamWeave Windowing Operations Example");
  println!("===========================================");
  println!();

  match example {
    "tumbling" => tumbling_window_example().await?,
    "sliding" => sliding_window_example().await?,
    "count" => count_window_example().await?,
    _ => {
      println!("Usage: cargo run --example windowing [tumbling|sliding|count]");
      return Ok(());
    }
  }

  Ok(())
}
