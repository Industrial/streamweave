mod pipeline;

#[allow(unused_imports)]
use pipeline::{
  basic_query_example, connection_pooling_example, large_result_set_example,
  parameterized_query_example,
};

#[tokio::main]
#[cfg(feature = "database")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("basic");

  println!("🚀 StreamWeave Database Integration Example");
  println!("===========================================");
  println!();

  match example {
    "basic" => {
      println!("Running: Basic Database Query");
      println!("-----------------------------");
      basic_query_example().await?;
    }
    "parameterized" => {
      println!("Running: Parameterized Query");
      println!("----------------------------");
      parameterized_query_example().await?;
    }
    "large" => {
      println!("Running: Large Result Set Streaming");
      println!("----------------------------------");
      large_result_set_example().await?;
    }
    "pooling" => {
      println!("Running: Connection Pooling");
      println!("--------------------------");
      connection_pooling_example().await?;
    }
    _ => {
      println!(
        "Usage: cargo run --example database_integration --features database [basic|parameterized|large|pooling]"
      );
      println!();
      println!("Examples:");
      println!("  cargo run --example database_integration --features database basic");
      println!("  cargo run --example database_integration --features database parameterized");
      println!("  cargo run --example database_integration --features database large");
      println!("  cargo run --example database_integration --features database pooling");
      return Ok(());
    }
  }

  Ok(())
}

#[cfg(not(feature = "database"))]
fn main() {
  eprintln!("❌ Error: Database feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'database' feature to be enabled.");
  eprintln!("Build with: cargo run --example database_integration --features database");
  std::process::exit(1);
}
