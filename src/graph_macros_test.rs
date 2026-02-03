//! # Graph Macros Test Suite
//!
//! Comprehensive test suite for the `graph!` macro, including node definition parsing,
//! connection parsing, and graph construction.

use crate::graph;
use crate::graph::Graph;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};

// Mock nodes for testing
struct MockProducerNode {
  name: String,
  data: Vec<i32>,
  output_port_names: Vec<String>,
}

impl MockProducerNode {
  fn new(name: String, data: Vec<i32>) -> Self {
    Self {
      name,
      data,
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for MockProducerNode {
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

struct MockTransformNode {
  name: String,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
}

impl MockTransformNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for MockTransformNode {
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

struct MockSinkNode {
  name: String,
  input_port_names: Vec<String>,
}

impl MockSinkNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
    }
  }
}

#[async_trait]
impl Node for MockSinkNode {
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
    &[]
  }
  fn has_input_port(&self, name: &str) -> bool {
    name == "in"
  }
  fn has_output_port(&self, _name: &str) -> bool {
    false
  }
  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let _input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      Ok(HashMap::new())
    })
  }
}

#[tokio::test]
async fn test_graph_macro_simple_linear_pipeline() {
  // Test that graph! macro can create a simple linear pipeline
  // producer -> transform -> sink

  let mut graph: Graph = graph! {
      producer: MockProducerNode::new("producer".to_string(), vec![1, 2, 3]),
      transform: MockTransformNode::new("transform".to_string()),
      sink: MockSinkNode::new("sink".to_string()),
      producer.out => transform.in,
      transform.out => sink.in
  };

  // Verify graph structure
  assert_eq!(graph.node_count(), 3);
  assert_eq!(graph.edge_count(), 2);
  assert!(graph.has_node("producer"));
  assert!(graph.has_node("transform"));
  assert!(graph.has_node("sink"));

  // Verify edges exist
  assert!(
    graph
      .find_edge_by_nodes_and_ports("producer", "out", "transform", "in")
      .is_some()
  );
  assert!(
    graph
      .find_edge_by_nodes_and_ports("transform", "out", "sink", "in")
      .is_some()
  );

  // Execute the graph (use Graph::execute to disambiguate from Node::execute)
  assert!(Graph::execute(&mut graph).await.is_ok());

  // Wait for completion
  assert!(graph.wait_for_completion().await.is_ok());
}

#[test]
#[should_panic(expected = "Fan-out not supported")]
fn test_graph_macro_fan_out_rejected() {
  // Test that fan-out is rejected at build time via GraphBuilder
  // Attempting to connect the same source port twice should panic

  use crate::graph_builder::GraphBuilder;

  let builder = GraphBuilder::new("test")
    .add_node(
      "source",
      Box::new(MockProducerNode::new("source".to_string(), vec![1, 2, 3])),
    )
    .add_node(
      "filter1",
      Box::new(MockTransformNode::new("filter1".to_string())),
    )
    .add_node(
      "filter2",
      Box::new(MockTransformNode::new("filter2".to_string())),
    )
    .connect("source", "out", "filter1", "in")
    .connect("source", "out", "filter2", "in"); // This should panic: fan-out not supported

  let _graph = builder.build().unwrap();
}

#[tokio::test]
async fn test_graph_macro_graph_io_with_values() {
  // Test graph I/O with values: graph.input: value => node.port and node.port => graph.output

  // Build graph with I/O using macro (one connection at a time for now)
  let mut graph: Graph = graph! {
      transform: MockTransformNode::new("transform".to_string()),
      sink: MockSinkNode::new("sink".to_string()),
      graph.config: 42i32 => transform.in
  };

  // Add remaining connections manually
  graph
    .add_edge(crate::edge::Edge {
      source_node: "transform".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph
    .expose_output_port("transform", "out", "result")
    .unwrap();

  // Verify graph structure
  assert_eq!(graph.node_count(), 2);
  assert!(graph.has_node("transform"));
  assert!(graph.has_node("sink"));

  // Verify graph has input/output ports exposed
  assert!(graph.has_input_port("config"));
  assert!(graph.has_output_port("result"));

  // Note: The value (42i32) is sent via channel when graph is built
  // Execution with values requires external I/O setup which is tested elsewhere
  // This test verifies the macro correctly creates graph I/O bindings
}

#[tokio::test]
async fn test_graph_macro_graph_io_without_values() {
  // Test graph I/O without values: graph.input => node.port and node.port => graph.output

  let mut graph: Graph = graph! {
      transform: MockTransformNode::new("transform".to_string()),
      graph.config => transform.in
  };

  // Add output port manually
  graph
    .expose_output_port("transform", "out", "result")
    .unwrap();

  // Verify graph structure
  assert_eq!(graph.node_count(), 1);
  assert!(graph.has_node("transform"));

  // Verify graph has input/output ports exposed
  assert!(graph.has_input_port("config"));
  assert!(graph.has_output_port("result"));

  // Note: Without values, external channels must be connected at runtime
  // This test verifies the macro correctly creates graph I/O bindings without initial values
}

#[test]
#[should_panic(expected = "Node name 'graph' is reserved")]
fn test_graph_macro_invalid_graph_node_name() {
  // Test that using "graph" as a node name causes a compile error

  // This should fail at compile time with validate_node_name!
  let _graph: Graph = graph! {
      graph: MockProducerNode::new("graph".to_string(), vec![1, 2, 3])
  };
}

#[tokio::test]
async fn test_graph_macro_combined_patterns() {
  // Test combined patterns: linear pipeline + graph I/O
  // Note: Multiple connections in macro not yet fully supported, so we build incrementally

  let mut graph: Graph = graph! {
      producer: MockProducerNode::new("producer".to_string(), vec![1, 2, 3]),
      transform: MockTransformNode::new("transform".to_string()),
      sink: MockSinkNode::new("sink".to_string()),
      producer.out => transform.in,
      transform.out => sink.in
  };

  // Add graph I/O (note: graph.input with value must be done via builder, not post-build)
  graph
    .expose_input_port("transform", "in", "config")
    .unwrap();
  graph
    .expose_output_port("transform", "out", "output")
    .unwrap();

  // Verify graph structure
  assert_eq!(graph.node_count(), 3);
  assert!(graph.has_node("producer"));
  assert!(graph.has_node("transform"));
  assert!(graph.has_node("sink"));

  // Verify edges exist
  assert!(
    graph
      .find_edge_by_nodes_and_ports("producer", "out", "transform", "in")
      .is_some()
  );
  assert!(
    graph
      .find_edge_by_nodes_and_ports("transform", "out", "sink", "in")
      .is_some()
  );

  // Verify graph I/O ports
  assert!(graph.has_input_port("config"));
  assert!(graph.has_output_port("output"));

  // Execute the graph
  assert!(Graph::execute(&mut graph).await.is_ok());

  // Wait for completion
  assert!(graph.wait_for_completion().await.is_ok());
}
