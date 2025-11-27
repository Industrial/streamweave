mod pipeline;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use pipeline::run_server;

#[tokio::main]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🚀 StreamWeave HTTP Server Integration Example");
  println!("==============================================");
  println!();
  println!("This example demonstrates:");
  println!("  • HTTP server types (HttpRequest, HttpResponse, HttpMethod, ContentType)");
  println!("  • HTTP request producer");
  println!("  • HTTP response consumer");
  println!("  • Axum route handler integration");
  println!();
  println!("Starting HTTP server on http://127.0.0.1:3000");
  println!("Press Ctrl+C to stop");
  println!();

  run_server().await?;

  Ok(())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "http-server")))]
fn main() {
  eprintln!("❌ Error: HTTP server feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'http-server' feature to be enabled.");
  eprintln!("Build with: cargo run --example http_server_integration --features http-server");
  std::process::exit(1);
}
