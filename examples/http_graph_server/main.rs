//! # HTTP Graph Server Example
//!
//! This example demonstrates building a complete HTTP web framework using StreamWeave's
//! Graph API. All HTTP traffic flows through a single graph, enabling powerful patterns
//! like path-based routing, fan-out, fan-in, and complex request processing pipelines.
//!
//! ## Features Demonstrated
//!
//! 1. **Basic HTTP Graph Server** - Long-lived graph processing HTTP requests
//! 2. **Path-Based Routing** - Route requests to different handlers by URL path
//! 3. **Multiple Handler Types** - REST, GraphQL-style, RPC, Static files
//! 4. **Fan-Out Patterns** - One request processed by multiple handlers
//! 5. **Fan-In Patterns** - Multiple sources merged into one response
//! 6. **Request/Response Correlation** - Automatic matching of responses to requests
//! 7. **Error Handling** - Graceful error handling throughout the graph
//! 8. **Transformers in Graph** - Request transformation and validation

mod handlers;
mod server;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use server::create_graph_server;

#[tokio::main]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🚀 StreamWeave HTTP Graph Server Example");
  println!("========================================");
  println!();
  println!("This example demonstrates:");
  println!("  • Long-lived graph-based HTTP server");
  println!("  • Path-based routing to different handlers");
  println!("  • REST, GraphQL-style, RPC, and Static file handlers");
  println!("  • Fan-out and fan-in patterns");
  println!("  • Request/response correlation");
  println!("  • Error handling and transformers");
  println!();
  println!("Starting HTTP Graph Server on http://127.0.0.1:3000");
  println!("Press Ctrl+C to stop");
  println!();
  println!("Available endpoints:");
  println!("  GET  /api/rest/users          - REST API handler");
  println!("  POST /api/rest/users          - REST API handler");
  println!("  POST /api/graphql             - GraphQL-style handler");
  println!("  POST /api/rpc/echo            - RPC handler");
  println!("  GET  /static/*                - Static file handler");
  println!("  GET  /api/fanout              - Fan-out example");
  println!("  GET  /api/health              - Health check");
  println!();

  create_graph_server().await?;

  Ok(())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "http-server")))]
fn main() {
  eprintln!("❌ Error: HTTP server feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'http-server' feature to be enabled.");
  eprintln!("Build with: cargo run --example http_graph_server --features http-server");
  std::process::exit(1);
}
