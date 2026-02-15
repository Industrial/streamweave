//! # Bounded iteration (cyclic dataflow)
//!
//! Demonstrates [`BoundedIterationNode`]: run an inner graph for up to `max_rounds`;
//! round 1 uses seed input, rounds 2..N feed previous output back as feedback.
//!
//! See [docs/cyclic-iterative-dataflows.md](docs/cyclic-iterative-dataflows.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::bounded_iteration_node::BoundedIterationNode;
use streamweave::nodes::stream::MergeNode;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};

fn build_inner_graph() -> Graph {
    let mut g = Graph::new("inner".to_string());
    let merge = MergeNode::new_deterministic("merge".to_string(), 2);
    g.add_node("merge".to_string(), Box::new(merge)).unwrap();
    g.expose_input_port("merge", "in_0", "seed").unwrap();
    g.expose_input_port("merge", "in_1", "feedback").unwrap();
    g.expose_output_port("merge", "out", "output").unwrap();
    g
}

/// Producer that emits a single i32 (seed for iteration).
struct SeedNode {
    name: String,
    value: i32,
    output_port_names: Vec<String>,
}

impl SeedNode {
    fn new(name: String, value: i32) -> Self {
        Self {
            name,
            value,
            output_port_names: vec!["out".to_string()],
        }
    }
}

#[async_trait]
impl Node for SeedNode {
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
    println!("Bounded iteration example (BoundedIterationNode, max_rounds)\n");

    let inner = build_inner_graph();
    let iter_node = BoundedIterationNode::new("iter".to_string(), inner, 3);

    let mut graph = Graph::new("bounded_iter".to_string());
    graph
        .add_node("seed".to_string(), Box::new(SeedNode::new("seed".to_string(), 5)))
        .unwrap();
    graph
        .add_node("iter".to_string(), Box::new(iter_node))
        .unwrap();
    graph
        .add_edge(Edge {
            source_node: "seed".to_string(),
            source_port: "out".to_string(),
            target_node: "iter".to_string(),
            target_port: "seed".to_string(),
        })
        .unwrap();
    graph.expose_output_port("iter", "output", "output").unwrap();

    let (tx, mut rx) = mpsc::channel(10);
    graph.connect_output_channel("output", tx).unwrap();

    Graph::execute(&mut graph).await?;

    let mut out = Vec::new();
    while let Some(arc) = rx.recv().await {
        if let Ok(n) = arc.downcast::<i32>() {
            out.push(*n);
        }
    }
    graph.wait_for_completion().await?;

    println!("Output (seed=5, max_rounds=3): {:?}", out);
    assert_eq!(out.len(), 3, "one value per round");
    assert!(out.iter().all(|&x| x == 5));
    println!("\nDone.");
    Ok(())
}
