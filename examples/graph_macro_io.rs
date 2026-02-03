//! # Graph I/O Patterns Example
//!
//! This example demonstrates the `graph!` macro with graph-level input and output:
//! - Graph inputs with initial values: `graph.input: value => node.port`
//! - Graph inputs without values: `graph.input => node.port`
//! - Graph outputs: `node.port => graph.output`
//!
//! Graph I/O allows you to expose internal node ports as graph-level interfaces,
//! making graphs composable and reusable.

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::{Stream, StreamExt};

// Transform node that processes input
struct TransformNode {
  name: String,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
}

impl TransformNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for TransformNode {
  fn name(&self) -> &str {
    &self.name
  }
  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }
  fn input_port_names(&self) -> &[String] {
    &self.input_port_names
  }
  fn output_port_names(&self) -> &[String] {
    &self.output_port_names
  }
  fn has_input_port(&self, name: &str) -> bool {
    name == "in"
  }
  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }
  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(async_stream::stream! {
            let mut input = input_stream;
            while let Some(item) = input.next().await {
                yield item;
            }
        }) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Graph I/O Patterns Example");
  println!("===========================");
  println!("\nThis example demonstrates graph-level input and output ports.");

  // Example 1: Graph input with initial value
  println!("\n1. Graph input with initial value:");
  println!("   graph.config: 42i32 => transform.in");

  let mut graph1: Graph = graph! {
      transform: TransformNode::new("transform".to_string()),
      graph.config: 42i32 => transform.in
  };

  graph1
    .expose_output_port("transform", "out", "result")
    .map_err(|e| format!("Failed to expose output: {}", e))?;

  println!("   ✓ Graph created with input port 'config' (value: 42)");
  println!("   ✓ Graph has output port 'result'");
  assert!(graph1.has_input_port("config"));
  assert!(graph1.has_output_port("result"));

  // Example 2: Graph input without value (for runtime connection)
  println!("\n2. Graph input without value:");
  println!("   graph.input => transform.in");

  let mut graph2: Graph = graph! {
      transform: TransformNode::new("transform".to_string()),
      graph.input => transform.in
  };

  graph2
    .expose_output_port("transform", "out", "output")
    .map_err(|e| format!("Failed to expose output: {}", e))?;

  println!("   ✓ Graph created with input port 'input' (no initial value)");
  println!("   ✓ Graph has output port 'output'");
  assert!(graph2.has_input_port("input"));
  assert!(graph2.has_output_port("output"));

  // Example 3: Graph output
  println!("\n3. Graph output:");
  println!("   transform.out => graph.result");

  let graph3: Graph = graph! {
      transform: TransformNode::new("transform".to_string()),
      transform.out => graph.result
  };

  println!("   ✓ Graph created with output port 'result'");
  assert!(graph3.has_output_port("result"));

  println!("\n✓ All graph I/O patterns demonstrated successfully!");
  println!("\nNote: Graph inputs with values send data via channels when the graph is built.");
  println!("Graph inputs without values require external channels to be connected at runtime.");

  Ok(())
}
