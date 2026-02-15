//! # Deterministic execution
//!
//! Demonstrates reproducible runs with [`execute_deterministic`] and
//! [`MergeNode::new_deterministic`]. Same graph run twice yields identical output order.
//!
//! See [docs/deterministic-execution.md](docs/deterministic-execution.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::stream::MergeNode;
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

/// Producer that emits a fixed sequence of i32 values.
struct ProducerNode {
  name: String,
  data: Vec<i32>,
  output_port_names: Vec<String>,
}

impl ProducerNode {
  fn new(name: String, data: Vec<i32>) -> Self {
    Self {
      name,
      data,
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
    let data = self.data.clone();
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(10);
      tokio::spawn(async move {
        for x in data {
          let _ = tx.send(Arc::new(x) as Arc<dyn Any + Send + Sync>).await;
        }
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

async fn run_once() -> Result<Vec<i32>, Box<dyn std::error::Error + Send + Sync>> {
  let mut graph = Graph::new("det".to_string());
  graph
    .add_node(
      "a".to_string(),
      Box::new(ProducerNode::new("a".to_string(), vec![1, 2, 3])),
    )
    .unwrap();
  graph
    .add_node(
      "b".to_string(),
      Box::new(ProducerNode::new("b".to_string(), vec![10, 20])),
    )
    .unwrap();
  graph
    .add_node(
      "merge".to_string(),
      Box::new(MergeNode::new_deterministic("merge".to_string(), 2)),
    )
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "merge".to_string(),
      target_port: "in_0".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "b".to_string(),
      source_port: "out".to_string(),
      target_node: "merge".to_string(),
      target_port: "in_1".to_string(),
    })
    .unwrap();
  graph.expose_output_port("merge", "out", "output").unwrap();

  let (tx, mut rx) = mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  graph.execute_deterministic().await?;

  let mut out = Vec::new();
  while let Some(arc) = rx.recv().await {
    if let Ok(x) = arc.downcast::<i32>() {
      out.push(*x);
    }
  }
  graph.wait_for_completion().await?;
  Ok(out)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!(
    "Deterministic execution example (execute_deterministic, MergeNode::new_deterministic)\n"
  );

  let run1 = run_once().await?;
  let run2 = run_once().await?;

  println!("Run 1: {:?}", run1);
  println!("Run 2: {:?}", run2);
  assert_eq!(
    run1, run2,
    "deterministic mode must produce identical outputs"
  );
  println!("\nBoth runs identical. Done.");
  Ok(())
}
