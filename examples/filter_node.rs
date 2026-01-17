//! # FilterNode Example
//!
//! This example demonstrates how to use FilterNode to filter data based on a condition.
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example filter-node
//! ```

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::{FilterNode, filter_config, stream::DropNode};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;

// Minimal source node
struct SourceNode {
  name: String,
  data: Vec<i32>,
}

#[async_trait]
impl Node for SourceNode {
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
    &[]
  }
  fn has_input_port(&self, _: &str) -> bool {
    false
  }
  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }
  fn execute(
    &self,
    _: InputStreams,
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
      Ok(HashMap::from([(
        "out".to_string(),
        Box::pin(ReceiverStream::new(rx))
          as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      )]))
    })
  }
}

// Config source
struct ConfigSource {
  name: String,
  config: streamweave::nodes::FilterConfig,
}

#[async_trait]
impl Node for ConfigSource {
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
    &[]
  }
  fn has_input_port(&self, _: &str) -> bool {
    false
  }
  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }
  fn execute(
    &self,
    _: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let config = Arc::clone(&self.config);
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(1);
      let config_wrapped: Arc<streamweave::nodes::FilterConfig> = Arc::new(config);
      let _ = tx.send(config_wrapped as Arc<dyn Any + Send + Sync>).await;
      Ok(HashMap::from([(
        "out".to_string(),
        Box::pin(ReceiverStream::new(rx))
          as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      )]))
    })
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("FilterNode Example");
  println!("==================");

  let mut graph = streamweave::graph::Graph::new("filter_example".to_string());

  // Create nodes
  let source = Box::new(SourceNode {
    name: "source".to_string(),
    data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  });
  let filter = Box::new(FilterNode::new("filter".to_string()));
  let sink = Box::new(DropNode::new("sink".to_string()));
  let config = Box::new(ConfigSource {
    name: "config".to_string(),
    config: filter_config(|v| async move {
      if let Ok(arc_i32) = v.downcast::<i32>() {
        Ok(*arc_i32 % 2 == 0) // Filter even numbers
      } else {
        Ok(false)
      }
    }),
  });

  // Add nodes
  graph.add_node("source".to_string(), source)?;
  graph.add_node("filter".to_string(), filter)?;
  graph.add_node("sink".to_string(), sink)?;
  graph.add_node("config".to_string(), config)?;

  // Connect edges
  graph.add_edge(Edge {
    source_node: "source".to_string(),
    source_port: "out".to_string(),
    target_node: "filter".to_string(),
    target_port: "in".to_string(),
  })?;
  graph.add_edge(Edge {
    source_node: "config".to_string(),
    source_port: "out".to_string(),
    target_node: "filter".to_string(),
    target_port: "configuration".to_string(),
  })?;
  graph.add_edge(Edge {
    source_node: "filter".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
  })?;

  println!(
    "Graph: {} nodes, {} edges",
    graph.get_nodes().len(),
    graph.get_edges().len()
  );
  println!("Processing: [1..10] -> Filter(even) -> Drop");

  // Execute
  graph
    .execute()
    .await
    .map_err(|e| format!("Execution error: {}", e))?;
  graph
    .wait_for_completion()
    .await
    .map_err(|e| format!("Completion error: {}", e))?;

  println!("Graph execution completed!");
  Ok(())
}
