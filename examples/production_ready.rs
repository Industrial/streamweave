//! # Production readiness (health, liveness, Prometheus, logging)
//!
//! Demonstrates:
//! - **is_ready / is_live**: Before execution the graph is live but not ready;
//!   after execution starts it becomes ready; after completion it is no longer ready
//!   (use for Kubernetes readiness/liveness probes).
//! - **Prometheus**: Metrics recorder installed; `/metrics` exposed for scraping.
//! - **Logging**: Structured logging via `tracing` (init with `tracing_subscriber` in production).
//!
//! See [docs/production-cluster-tooling.md](../docs/production-cluster-tooling.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::metrics;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};

/// Producer that emits one value then ends.
struct ProducerNode {
    name: String,
    value: i32,
    output_port_names: Vec<String>,
}

impl ProducerNode {
    fn new(name: String, value: i32) -> Self {
        Self {
            name,
            value,
            output_port_names: vec!["out".to_string()],
        }
    }
}

#[async_trait]
impl Node for ProducerNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    fn input_port_names(&self) -> &[String] {
        &[]
    }
    fn output_port_names(&self) -> &[String] {
        &self.output_port_names
    }
    fn has_input_port(&self, _name: &str) -> bool {
        false
    }
    fn has_output_port(&self, name: &str) -> bool {
        name == "out"
    }
    fn execute(
        &self,
        _inputs: InputStreams,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
    > {
        let value = self.value;
        Box::pin(async move {
            let (tx, rx) = mpsc::channel(10);
            tokio::spawn(async move {
                let _ = tx.send(Arc::new(value) as Arc<dyn Any + Send + Sync>).await;
            });
            let mut outputs = HashMap::new();
            outputs.insert(
                "out".to_string(),
                Box::pin(ReceiverStream::new(rx))
                    as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
            );
            Ok(outputs)
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Prometheus: expose /metrics for scraping (e.g. Kubernetes, Prometheus server).
    let metrics_addr: SocketAddr = "127.0.0.1:9091".parse()?;
    metrics::install_prometheus_recorder_on(metrics_addr);
    println!("Prometheus metrics at http://{}/metrics\n", metrics_addr);

    println!("Production readiness example (is_ready, is_live, Prometheus)\n");

    let mut graph = Graph::new("health_demo".to_string());
    graph
        .add_node("producer".to_string(), Box::new(ProducerNode::new("producer".to_string(), 42)))
        .unwrap();
    graph.expose_output_port("producer", "out", "output").unwrap();

    let (tx, mut rx) = mpsc::channel(10);
    graph.connect_output_channel("output", tx).unwrap();

    println!("Before execute: is_ready={}, is_live={}", graph.is_ready(), graph.is_live());

    Graph::execute(&mut graph).await?;
    println!("After execute: is_ready={}, is_live={}", graph.is_ready(), graph.is_live());

    if let Some(arc) = rx.recv().await {
        // Example: record throughput for Prometheus (production nodes would call metrics in execute).
        metrics::record_items_out("health_demo", "producer", "out", 1);
        let _ = arc;
    }
    graph.wait_for_completion().await?;
    println!("After wait_for_completion: is_ready={}, is_live={}", graph.is_ready(), graph.is_live());

    println!("\nDone. In production, use tracing + tracing_subscriber for structured logging.");
    Ok(())
}
