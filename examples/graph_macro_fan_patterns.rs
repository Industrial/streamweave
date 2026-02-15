//! # Fan-In Pattern Example
//!
//! This example demonstrates the `graph!` macro with fan-in patterns:
//! multiple source nodes feeding into a single target node.
//!
//! The graph structure is also defined in [graph_macro_fan_patterns.mmd](graph_macro_fan_patterns.mmd)
//! (Style B Mermaid diagram).
//!
//! **Note**: Fan-out (one source feeding multiple targets) is not currently supported.
//! Each output port can only connect to one input port.
//!
//! The `graph!` macro provides a concise, declarative syntax for building graphs,
//! reducing boilerplate compared to the traditional Graph API.

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;

// Simple producer node that emits numbers
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
        for item in data {
          let _ = tx.send(Arc::new(item) as Arc<dyn Any + Send + Sync>).await;
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

// Merge node that combines multiple inputs
struct MergeNode {
  name: String,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
}

impl MergeNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in1".to_string(), "in2".to_string(), "in3".to_string()],
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for MergeNode {
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
    self.input_port_names.contains(&name.to_string())
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
      // Merge all input streams
      let mut streams = Vec::new();
      for port in &["in1", "in2", "in3"] {
        if let Some(stream) = inputs.remove(*port) {
          streams.push(stream);
        }
      }

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(async_stream::stream! {
            // Interleave items from all input streams
            use futures::stream;
            use tokio_stream::StreamExt;
            let merged = stream::select_all(streams);
            let mut merged = Box::pin(merged);
            while let Some(item) = StreamExt::next(&mut merged).await {
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
  println!("Fan-In Pattern Example");
  println!("======================");
  println!("\nThis example demonstrates fan-in: multiple sources feeding one target.");
  println!("Note: Fan-out (one source to multiple targets) is not supported.");

  // Create graph with fan-in pattern: source1, source2, source3 -> merge
  let mut graph: Graph = graph! {
      source1: ProducerNode::new("source1".to_string(), vec![1, 2, 3]),
      source2: ProducerNode::new("source2".to_string(), vec![4, 5, 6]),
      source3: ProducerNode::new("source3".to_string(), vec![7, 8, 9]),
      merge: MergeNode::new("merge".to_string()),
      source1.out => merge.in1,
      source2.out => merge.in2,
      source3.out => merge.in3
  };

  println!("\nGraph structure:");
  println!("  - source1: emits [1, 2, 3]");
  println!("  - source2: emits [4, 5, 6]");
  println!("  - source3: emits [7, 8, 9]");
  println!("  - merge: combines all inputs");

  // Execute the graph (use Graph::execute to disambiguate from Node::execute)
  println!("\nExecuting graph...");
  Graph::execute(&mut graph)
    .await
    .map_err(|e| format!("Graph execution failed: {}", e))?;
  graph
    .wait_for_completion()
    .await
    .map_err(|e| format!("Graph completion failed: {}", e))?;

  println!("✓ Graph execution completed");
  println!("\nThe graph merged data from three sources:");
  println!("  - source1: [1, 2, 3]");
  println!("  - source2: [4, 5, 6]");
  println!("  - source3: [7, 8, 9]");
  println!("  - merge: interleaved all items");
  println!("\n✓ Example completed successfully!");

  Ok(())
}
