mod pipeline;

use pipeline::{csv_example, jsonl_example, parquet_example};

#[tokio::main]
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();
  let example = args.get(1).map(|s| s.as_str()).unwrap_or("csv");

  println!("🚀 StreamWeave File Formats Integration Example");
  println!("================================================");
  println!();

  match example {
    "csv" => {
      println!("Running: CSV Read/Write Example");
      println!("-------------------------------");
      csv_example().await?;
    }
    "jsonl" => {
      println!("Running: JSONL Streaming Example");
      println!("--------------------------------");
      jsonl_example().await?;
    }
    "parquet" => {
      println!("Running: Parquet Column Projection Example");
      println!("------------------------------------------");
      parquet_example().await?;
    }
    _ => {
      println!(
        "Usage: cargo run --example file_formats --features file-formats [csv|jsonl|parquet]"
      );
      println!();
      println!("Examples:");
      println!("  cargo run --example file_formats --features file-formats csv");
      println!("  cargo run --example file_formats --features file-formats jsonl");
      println!("  cargo run --example file_formats --features file-formats parquet");
      return Ok(());
    }
  }

  Ok(())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "file-formats")))]
fn main() {
  eprintln!("❌ Error: File formats feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'file-formats' feature to be enabled.");
  eprintln!("Build with: cargo run --example file_formats --features file-formats");
  std::process::exit(1);
}

