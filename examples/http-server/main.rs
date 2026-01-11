//! # HTTP Server Example
//!
//! This example demonstrates how to create an HTTP server using StreamWeave's graph-based
//! architecture. It shows:
//!
//! 1. Building a graph with `HttpServerProducerNode`, `HttpPathRouterNode`, and `HttpServerConsumerNode`
//! 2. Configuring path-based routing to route requests to different handlers
//! 3. Using `GraphServer` to serve HTTP requests
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example http-server
//! ```
//!
//! The server will start on `http://0.0.0.0:3000`. You can test it with:
//!
//! ```bash
//! curl http://localhost:3000/
//! curl -X POST http://localhost:3000/api/echo -d '{"message": "Hello"}'
//! ```
//!
//! ## About Box::new()
//!
//! The `GraphBuilder::node()` method requires `Box<dyn NodeTrait + Send + Sync>` because:
//! - The graph stores nodes as trait objects for dynamic dispatch
//! - Different node types (ProducerNode, TransformerNode, ConsumerNode, etc.) need to be stored
//!   in the same data structure
//! - `Box` allocates the node on the heap and provides the trait object pointer
//! - The `as Box<dyn NodeTrait + Send + Sync>` cast is unnecessary but harmless - `Box::new()`
//!   already converts the concrete type to the trait object type

use std::net::SocketAddr;
use streamweave::graph::GraphBuilder;
use streamweave::graph::http_server::nodes::{
  HttpPathRouterNode, HttpServerConsumerNode, HttpServerProducerNode,
};
use streamweave::graph::http_server::{GraphServer, GraphServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize tracing
  tracing_subscriber::fmt::init();

  tracing::info!("Starting HTTP server example...");

  // Create HTTP server nodes
  let producer_node = HttpServerProducerNode::with_default_config("http_producer".to_string());

  // Create path router node and configure routes
  let mut router_node = HttpPathRouterNode::with_default_config("path_router".to_string());
  // Route all paths starting with "/api/" to port 0
  router_node.add_route("/api/*".to_string(), 0);
  // Route root path "/" to port 1
  router_node.add_route("/".to_string(), 1);
  // Set default port for unmatched routes (optional - if not set, unmatched routes are dropped)
  router_node.set_default_port(Some(1));

  let consumer_node = HttpServerConsumerNode::with_default_config("http_consumer".to_string());

  // Build the graph with routing:
  // http_producer -> path_router -> http_consumer
  // The router routes requests to different output ports (out_0, out_1, etc.)
  // For this simple example, we connect all router outputs to the same consumer
  let graph = GraphBuilder::new()
    // GraphBuilder::node() requires Box<dyn NodeTrait + Send + Sync> because the graph
    // stores nodes as trait objects for dynamic dispatch. Box::new() converts the concrete
    // type to a heap-allocated trait object.
    .node(Box::new(producer_node))?
    .node(Box::new(router_node))?
    .node(Box::new(consumer_node))?
    // Connect producer output to router input
    .connect_by_name("http_producer", "path_router")?
    // Connect router output port 0 (for /api/* routes) to consumer
    .connect("path_router", "out_0", "http_consumer", "in")?
    // Connect router output port 1 (for / routes) to consumer
    .connect("path_router", "out_1", "http_consumer", "in")?
    .build();

  tracing::info!("Graph built successfully with {} nodes", graph.node_count());

  // Create GraphServer from the graph
  let server = GraphServer::from_graph_with_node_names(
    graph,
    "http_producer".to_string(),
    "http_consumer".to_string(),
    GraphServerConfig::default(),
  )
  .await?;

  // Start the server
  let addr: SocketAddr = "0.0.0.0:3000".parse()?;
  tracing::info!("Starting HTTP server on {}", addr);
  tracing::info!("Server is ready to accept requests!");
  tracing::info!("Try: curl http://localhost:3000/");

  // This will block and keep the server running
  server.serve(addr).await?;

  Ok(())
}
