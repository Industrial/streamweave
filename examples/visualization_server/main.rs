//! # Visualization HTTP Server Example
//!
//! This example demonstrates how to serve pipeline visualizations via HTTP.
//! It creates a web server that serves DAG visualizations and provides
//! an API endpoint for DAG data.
//!
//! ## Features
//!
//! - HTTP server startup with configurable address
//! - DAG serving via API endpoint
//! - UI serving (HTML visualization)
//! - Graceful shutdown support
//! - Console output with server URL

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
mod pipeline;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use pipeline::create_multi_stage_pipeline;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::{
  Router,
  body::Body,
  http::{Response, StatusCode},
  response::IntoResponse,
  routing::get,
};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::net::SocketAddr;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::sync::Arc;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave::visualization::{
  DagEdge, DagExporter, DagNode, NodeKind, NodeMetadata, PipelineDag, generate_standalone_html,
};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use tokio::signal;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use tokio::sync::RwLock;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
/// Creates a DAG from a multi-stage pipeline manually.
fn create_dag_from_multi_stage_pipeline(
  producer: &impl streamweave::Producer<Output = i32>,
  transformer1: &impl streamweave::Transformer<Input = i32, Output = i32>,
  transformer2: &impl streamweave::Transformer<Input = i32, Output = i32>,
  consumer: &impl streamweave::Consumer<Input = i32>,
) -> PipelineDag {
  let mut dag = PipelineDag::new();

  // Create producer node
  let producer_info = producer.component_info();
  let producer_config = producer.config();
  let producer_metadata = NodeMetadata {
    component_type: producer_info.type_name,
    name: producer_config.name(),
    input_type: None,
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", producer_config.error_strategy()),
    custom: std::collections::HashMap::new(),
  };
  let producer_node = DagNode::new(
    "producer".to_string(),
    NodeKind::Producer,
    producer_metadata,
  );
  dag.add_node(producer_node);

  // Create first transformer node
  let transformer1_info = transformer1.component_info();
  let transformer1_config = transformer1.config();
  let transformer1_metadata = NodeMetadata {
    component_type: transformer1_info.type_name,
    name: transformer1_config.name(),
    input_type: Some("i32".to_string()),
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", transformer1_config.error_strategy()),
    custom: std::collections::HashMap::new(),
  };
  let transformer1_node = DagNode::new(
    "transformer1".to_string(),
    NodeKind::Transformer,
    transformer1_metadata,
  );
  dag.add_node(transformer1_node);

  // Create second transformer node
  let transformer2_info = transformer2.component_info();
  let transformer2_config = transformer2.config();
  let transformer2_metadata = NodeMetadata {
    component_type: transformer2_info.type_name,
    name: transformer2_config.name(),
    input_type: Some("i32".to_string()),
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", transformer2_config.error_strategy()),
    custom: std::collections::HashMap::new(),
  };
  let transformer2_node = DagNode::new(
    "transformer2".to_string(),
    NodeKind::Transformer,
    transformer2_metadata,
  );
  dag.add_node(transformer2_node);

  // Create consumer node
  let consumer_info = consumer.component_info();
  let consumer_config = consumer.config();
  let consumer_metadata = NodeMetadata {
    component_type: consumer_info.type_name,
    name: Some(consumer_config.name.clone()),
    input_type: Some("i32".to_string()),
    output_type: None,
    error_strategy: format!("{:?}", consumer_config.error_strategy),
    custom: std::collections::HashMap::new(),
  };
  let consumer_node = DagNode::new(
    "consumer".to_string(),
    NodeKind::Consumer,
    consumer_metadata,
  );
  dag.add_node(consumer_node);

  // Create edges
  dag.add_edge(DagEdge::new(
    "producer".to_string(),
    "transformer1".to_string(),
    Some("i32".to_string()),
  ));
  dag.add_edge(DagEdge::new(
    "transformer1".to_string(),
    "transformer2".to_string(),
    Some("i32".to_string()),
  ));
  dag.add_edge(DagEdge::new(
    "transformer2".to_string(),
    "consumer".to_string(),
    Some("i32".to_string()),
  ));

  dag
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Visualization HTTP Server Example");
  println!("================================================");
  println!();

  // Create pipeline components
  let (producer, transformer1, transformer2, consumer) = create_multi_stage_pipeline();

  // Generate DAG representation
  println!("📊 Generating pipeline DAG...");
  let dag =
    create_dag_from_multi_stage_pipeline(&producer, &transformer1, &transformer2, &consumer);

  println!("✅ DAG generated successfully!");
  println!("   Nodes: {}", dag.nodes().len());
  println!("   Edges: {}", dag.edges().len());
  println!();

  // Store DAG in shared state
  let dag_state = Arc::new(RwLock::new(dag));

  // Create router
  let app = Router::new()
    .route("/", get(serve_ui))
    .route("/api/dag", get(serve_dag_json))
    .route("/api/dag/dot", get(serve_dag_dot))
    .with_state(dag_state);

  // Start server
  let addr: SocketAddr = "127.0.0.1:8080".parse()?;
  let listener = tokio::net::TcpListener::bind(addr).await?;

  println!("🌐 Starting HTTP server...");
  println!("   Server address: http://{}", addr);
  println!("   UI: http://{}/", addr);
  println!("   DAG API: http://{}/api/dag", addr);
  println!("   DAG DOT: http://{}/api/dag/dot", addr);
  println!();
  println!("💡 Press Ctrl+C to stop the server");
  println!();

  // Start server with graceful shutdown
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;

  println!();
  println!("✅ Server stopped gracefully");

  Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
/// Serves the visualization UI (HTML)
async fn serve_ui(
  axum::extract::State(dag_state): axum::extract::State<Arc<RwLock<PipelineDag>>>,
) -> impl IntoResponse {
  let dag = dag_state.read().await;
  let html = generate_standalone_html(&dag);

  Response::builder()
    .status(StatusCode::OK)
    .header("Content-Type", "text/html; charset=utf-8")
    .body(Body::from(html))
    .unwrap()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
/// Serves the DAG as JSON
async fn serve_dag_json(
  axum::extract::State(dag_state): axum::extract::State<Arc<RwLock<PipelineDag>>>,
) -> impl IntoResponse {
  let dag = dag_state.read().await;
  match dag.to_json() {
    Ok(json) => Response::builder()
      .status(StatusCode::OK)
      .header("Content-Type", "application/json")
      .body(Body::from(json))
      .unwrap(),
    Err(e) => Response::builder()
      .status(StatusCode::INTERNAL_SERVER_ERROR)
      .body(Body::from(format!("Error: {}", e)))
      .unwrap(),
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
/// Serves the DAG as DOT format
async fn serve_dag_dot(
  axum::extract::State(dag_state): axum::extract::State<Arc<RwLock<PipelineDag>>>,
) -> impl IntoResponse {
  let dag = dag_state.read().await;
  let dot = dag.to_dot();

  Response::builder()
    .status(StatusCode::OK)
    .header("Content-Type", "text/plain; charset=utf-8")
    .body(Body::from(dot))
    .unwrap()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
/// Handles graceful shutdown signal
async fn shutdown_signal() {
  let ctrl_c = async {
    signal::ctrl_c()
      .await
      .expect("failed to install Ctrl+C handler");
  };

  #[cfg(unix)]
  let terminate = async {
    signal::unix::signal(signal::unix::SignalKind::terminate())
      .expect("failed to install signal handler")
      .recv()
      .await;
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
    _ = ctrl_c => {},
    _ = terminate => {},
  }

  println!();
  println!("🛑 Shutdown signal received, stopping server...");
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "http-server")))]
fn main() {
  println!("❌ HTTP server feature is not enabled");
  println!();
  println!("📦 To enable this example:");
  println!("   1. Enable the 'http-server' feature:");
  println!("      cargo run --example visualization_server --features http-server");
  println!();
  println!("   2. Or add to Cargo.toml:");
  println!("      [features]");
  println!("      default = [\"http-server\"]");
  println!();
  println!("💡 Requirements:");
  println!("   • http-server feature must be enabled");
  println!("   • axum HTTP framework (included with http-server feature)");
  println!("   • tokio async runtime");
}
