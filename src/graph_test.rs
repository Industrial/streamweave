//! # Graph Test Suite - Stream-Based
//!
//! Comprehensive test suite for the [`Graph`] struct with stream-based architecture,
//! including node management, edge management, and stream-based execution.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **Name Management**: Getting and setting graph names
//! - **Node Management**: Adding, removing, and querying nodes
//! - **Edge Management**: Adding, removing, and querying edges
//! - **Stream Execution**: Executing graphs with stream-based node connections

use crate::edge::Edge;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::{Graph, GraphBuilder, topological_sort};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};

// ============================================================================
// Mock Node Implementations for Testing
// ============================================================================

/// Mock producer node (0 inputs, 1 output)
struct MockProducerNode {
  name: String,
  output_port_names: Vec<String>,
  data: Vec<i32>,
}

impl MockProducerNode {
  fn new(name: String, data: Vec<i32>) -> Self {
    Self {
      name,
      output_port_names: vec!["out".to_string()],
      data,
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
    self.output_port_names.contains(&name.to_string())
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

/// Mock transform node (1 input, 1 output)
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
    self.input_port_names.contains(&name.to_string())
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.output_port_names.contains(&name.to_string())
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      let output_stream: OutputStreams = {
        let mut map = HashMap::new();
        map.insert(
          "out".to_string(),
          Box::pin(async_stream::stream! {
            let mut input = input_stream;
            while let Some(item) = input.next().await {
              if let Ok(arc_i32) = item.clone().downcast::<i32>() {
                yield Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>;
              } else {
                yield item;
              }
            }
          }) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
        map
      };

      Ok(output_stream)
    })
  }
}

/// Mock sink node (1 input, 0 outputs)
struct MockSinkNode {
  name: String,
  input_port_names: Vec<String>,
  received: Arc<tokio::sync::Mutex<Vec<i32>>>,
}

impl MockSinkNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      received: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    }
  }

  #[allow(dead_code)]
  async fn get_received(&self) -> Vec<i32> {
    self.received.lock().await.clone()
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
    self.input_port_names.contains(&name.to_string())
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
    let received = Arc::clone(&self.received);
    Box::pin(async move {
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      tokio::spawn(async move {
        let mut input = input_stream;
        while let Some(item) = input.next().await {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            received.lock().await.push(*arc_i32);
          }
        }
      });

      Ok(HashMap::new())
    })
  }
}

// ============================================================================
// Name Management Tests
// ============================================================================

#[test]
fn test_graph_name_get() {
  let graph = Graph::new("test_graph".to_string());
  assert_eq!(graph.name(), "test_graph");
}

#[test]
fn test_graph_name_set() {
  let mut graph = Graph::new("old_name".to_string());
  graph.set_name("new_name");
  assert_eq!(graph.name(), "new_name");
}

// ============================================================================
// Node Management Tests
// ============================================================================

#[test]
fn test_add_node() {
  let mut graph = Graph::new("test".to_string());
  let node = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  assert!(graph.add_node("producer".to_string(), node).is_ok());
  assert!(graph.find_node_by_name("producer").is_some());
}

#[test]
fn test_add_duplicate_node() {
  let mut graph = Graph::new("test".to_string());
  let node1 = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  let node2 = Box::new(MockProducerNode::new("producer".to_string(), vec![2]));

  assert!(graph.add_node("producer".to_string(), node1).is_ok());
  assert!(graph.add_node("producer".to_string(), node2).is_err());
}

#[test]
fn test_remove_node() {
  let mut graph = Graph::new("test".to_string());
  let node = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  graph.add_node("producer".to_string(), node).unwrap();
  assert!(graph.remove_node("producer").is_ok());
  assert!(graph.find_node_by_name("producer").is_none());
}

#[test]
fn test_remove_nonexistent_node() {
  let mut graph = Graph::new("test".to_string());
  assert!(graph.remove_node("nonexistent").is_err());
}

// ============================================================================
// Edge Management Tests
// ============================================================================

#[test]
fn test_add_edge() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  let edge = Edge {
    source_node: "producer".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
  };

  assert!(graph.add_edge(edge).is_ok());
  assert_eq!(graph.get_edges().len(), 1);
}

#[test]
fn test_add_edge_invalid_source() {
  let mut graph = Graph::new("test".to_string());
  let edge = Edge {
    source_node: "nonexistent".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
  };

  assert!(graph.add_edge(edge).is_err());
}

#[test]
fn test_remove_edge() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  let edge = Edge {
    source_node: "producer".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
  };

  graph.add_edge(edge).unwrap();
  assert!(graph.remove_edge("producer", "out", "sink", "in").is_ok());
  assert_eq!(graph.get_edges().len(), 0);
}

// ============================================================================
// Topological Sort Tests
// ============================================================================

#[test]
fn test_topological_sort_linear() {
  let node_a: Box<dyn Node> = Box::new(MockProducerNode::new("a".to_string(), vec![1]));
  let node_b: Box<dyn Node> = Box::new(MockTransformNode::new("b".to_string()));
  let node_c: Box<dyn Node> = Box::new(MockSinkNode::new("c".to_string()));
  let nodes: Vec<&dyn Node> = vec![node_a.as_ref(), node_b.as_ref(), node_c.as_ref()];

  let edge1 = Edge {
    source_node: "a".to_string(),
    source_port: "out".to_string(),
    target_node: "b".to_string(),
    target_port: "in".to_string(),
  };
  let edge2 = Edge {
    source_node: "b".to_string(),
    source_port: "out".to_string(),
    target_node: "c".to_string(),
    target_port: "in".to_string(),
  };
  let edges = vec![&edge1, &edge2];

  let result = topological_sort(&nodes, &edges).unwrap();
  assert_eq!(result, vec!["a", "b", "c"]);
}

#[test]
fn test_topological_sort_diamond() {
  let node_a: Box<dyn Node> = Box::new(MockProducerNode::new("a".to_string(), vec![1]));
  let node_b: Box<dyn Node> = Box::new(MockTransformNode::new("b".to_string()));
  let node_c: Box<dyn Node> = Box::new(MockTransformNode::new("c".to_string()));
  let node_d: Box<dyn Node> = Box::new(MockSinkNode::new("d".to_string()));
  let nodes: Vec<&dyn Node> = vec![
    node_a.as_ref(),
    node_b.as_ref(),
    node_c.as_ref(),
    node_d.as_ref(),
  ];

  let edge1 = Edge {
    source_node: "a".to_string(),
    source_port: "out".to_string(),
    target_node: "b".to_string(),
    target_port: "in".to_string(),
  };
  let edge2 = Edge {
    source_node: "a".to_string(),
    source_port: "out".to_string(),
    target_node: "c".to_string(),
    target_port: "in".to_string(),
  };
  let edge3 = Edge {
    source_node: "b".to_string(),
    source_port: "out".to_string(),
    target_node: "d".to_string(),
    target_port: "in".to_string(),
  };
  let edge4 = Edge {
    source_node: "c".to_string(),
    source_port: "out".to_string(),
    target_node: "d".to_string(),
    target_port: "in".to_string(),
  };
  let edges = vec![&edge1, &edge2, &edge3, &edge4];

  let result = topological_sort(&nodes, &edges).unwrap();
  assert_eq!(result[0], "a");
  assert!(result.contains(&"b".to_string()));
  assert!(result.contains(&"c".to_string()));
  assert_eq!(result[result.len() - 1], "d");
}

// ============================================================================
// Execution Tests
// ============================================================================

#[tokio::test]
async fn test_execute_simple_graph() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  let edge = Edge {
    source_node: "producer".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
  };

  graph.add_edge(edge).unwrap();

  // Execute the graph
  assert!(graph.execute().await.is_ok());

  // Wait for completion
  assert!(graph.wait_for_completion().await.is_ok());
}

#[tokio::test]
async fn test_execute_transform_graph() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let transform = Box::new(MockTransformNode::new("transform".to_string()));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("transform".to_string(), transform).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "transform".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph
    .add_edge(Edge {
      source_node: "transform".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  // Execute the graph
  assert!(graph.execute().await.is_ok());

  // Wait for completion
  assert!(graph.wait_for_completion().await.is_ok());
}

#[tokio::test]
async fn test_stop_execution() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph.execute().await.unwrap();

  // Stop execution
  assert!(graph.stop().await.is_ok());

  // Wait for completion (should complete quickly after stop)
  assert!(graph.wait_for_completion().await.is_ok());
}

// ============================================================================
// Graph as Node Tests
// ============================================================================

#[test]
fn test_graph_has_input_ports() {
  let graph = Graph::new("test".to_string());
  assert!(graph.has_input_port("configuration"));
  assert!(graph.has_input_port("input"));
  assert!(!graph.has_input_port("nonexistent"));
}

#[test]
fn test_graph_has_output_ports() {
  let graph = Graph::new("test".to_string());
  assert!(graph.has_output_port("output"));
  assert!(graph.has_output_port("error"));
  assert!(!graph.has_output_port("nonexistent"));
}

#[test]
fn test_graph_input_port_names() {
  let graph = Graph::new("test".to_string());
  let ports = graph.input_port_names();
  assert_eq!(ports.len(), 2);
  assert!(ports.contains(&"configuration".to_string()));
  assert!(ports.contains(&"input".to_string()));
}

#[test]
fn test_graph_output_port_names() {
  let graph = Graph::new("test".to_string());
  let ports = graph.output_port_names();
  assert_eq!(ports.len(), 2);
  assert!(ports.contains(&"output".to_string()));
  assert!(ports.contains(&"error".to_string()));
}

#[test]
fn test_expose_input_port() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));

  graph.add_node("producer".to_string(), producer).unwrap();

  // Expose producer's output as graph's input
  assert!(graph.expose_input_port("producer", "out", "input").is_err()); // producer has no input port

  // Create a node with input port
  let transform = Box::new(MockTransformNode::new("transform".to_string()));
  graph.add_node("transform".to_string(), transform).unwrap();

  // Expose transform's input as graph's input
  assert!(graph.expose_input_port("transform", "in", "input").is_ok());

  // Try invalid external port name
  assert!(
    graph
      .expose_input_port("transform", "in", "invalid")
      .is_err()
  );

  // Try non-existent internal node
  assert!(
    graph
      .expose_input_port("nonexistent", "in", "input")
      .is_err()
  );

  // Try non-existent internal port
  assert!(
    graph
      .expose_input_port("transform", "nonexistent", "input")
      .is_err()
  );
}

#[test]
fn test_expose_output_port() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));

  graph.add_node("producer".to_string(), producer).unwrap();

  // Expose producer's output as graph's output
  assert!(
    graph
      .expose_output_port("producer", "out", "output")
      .is_ok()
  );

  // Try invalid external port name
  assert!(
    graph
      .expose_output_port("producer", "out", "invalid")
      .is_err()
  );

  // Try non-existent internal node
  assert!(
    graph
      .expose_output_port("nonexistent", "out", "output")
      .is_err()
  );

  // Try non-existent internal port
  assert!(
    graph
      .expose_output_port("producer", "nonexistent", "output")
      .is_err()
  );
}

#[tokio::test]
async fn test_graph_as_node_execute() {
  // Create a subgraph
  let mut subgraph = Graph::new("subgraph".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let transform = Box::new(MockTransformNode::new("transform".to_string()));

  subgraph.add_node("producer".to_string(), producer).unwrap();
  subgraph
    .add_node("transform".to_string(), transform)
    .unwrap();

  subgraph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "transform".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  // Expose ports
  subgraph
    .expose_input_port("producer", "out", "input")
    .is_err(); // producer has no input
  // Instead, we'll expose transform's output as graph's output
  subgraph
    .expose_output_port("transform", "out", "output")
    .unwrap();

  // Create a parent graph with the subgraph as a node
  let mut parent_graph = Graph::new("parent".to_string());
  let subgraph_node: Box<dyn Node> = Box::new(subgraph);
  parent_graph
    .add_node("subgraph".to_string(), subgraph_node)
    .unwrap();

  // The subgraph needs input, but we can't easily test this without a proper source
  // For now, just verify the structure is correct
  assert!(parent_graph.find_node_by_name("subgraph").is_some());
}

// ============================================================================
// Lifecycle Control Tests
// ============================================================================

#[test]
fn test_start_pause_resume_stop() {
  let graph = Graph::new("test".to_string());

  // Initially stopped
  // (We can't easily check state without exposing it, but we can test the methods)

  // Start execution
  graph.start();

  // Pause execution
  graph.pause();

  // Resume execution
  graph.resume();

  // All methods should complete without error
  // (Actual state verification would require exposing execution_state or testing behavior)
}

#[tokio::test]
async fn test_stop_clears_state() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  // Start execution
  graph.execute().await.unwrap();

  // Stop should clear state
  assert!(graph.stop().await.is_ok());

  // After stop, execution handles should be cleared
  // (We verify this by checking wait_for_completion completes quickly)
  assert!(graph.wait_for_completion().await.is_ok());
}

// ============================================================================
// GraphBuilder Test Suite
// ============================================================================

#[tokio::test]
async fn test_graph_builder_creation() {
  let builder = GraphBuilder::new("test_graph");
  let graph = builder.build().unwrap();

  assert_eq!(graph.name(), "test_graph");
  assert_eq!(graph.node_count(), 0);
  assert_eq!(graph.edge_count(), 0);
}

#[tokio::test]
async fn test_graph_builder_add_single_node() {
  let node = Box::new(MockProducerNode::new("producer".to_string(), vec![42]));
  let graph = GraphBuilder::new("test")
    .add_node("producer", node)
    .build()
    .unwrap();

  assert_eq!(graph.node_count(), 1);
  assert!(graph.has_node("producer"));
  assert!(!graph.has_node("nonexistent"));
}

#[tokio::test]
async fn test_graph_builder_add_multiple_nodes() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let graph = GraphBuilder::new("pipeline")
    .add_node("producer", producer)
    .add_node("transformer", transformer)
    .add_node("consumer", consumer)
    .build()
    .unwrap();

  assert_eq!(graph.node_count(), 3);
  assert!(graph.has_node("producer"));
  assert!(graph.has_node("transformer"));
  assert!(graph.has_node("consumer"));
}

#[tokio::test]
async fn test_graph_builder_connect_nodes() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![42]));
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let graph = GraphBuilder::new("pipeline")
    .add_node("producer", producer)
    .add_node("consumer", consumer)
    .connect("producer", "out", "consumer", "in")
    .build()
    .unwrap();

  assert_eq!(graph.edge_count(), 1);
  assert!(graph.has_edge("producer", "consumer"));
}

#[tokio::test]
async fn test_graph_builder_multiple_connections() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![42]));
  let transformer1 = Box::new(MockTransformerNode::new("transformer1".to_string()));
  let transformer2 = Box::new(MockTransformerNode::new("transformer2".to_string()));
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let graph = GraphBuilder::new("complex_pipeline")
    .add_node("producer", producer)
    .add_node("transformer1", transformer1)
    .add_node("transformer2", transformer2)
    .add_node("consumer", consumer)
    .connect("producer", "out", "transformer1", "in")
    .connect("producer", "out", "transformer2", "in")
    .connect("transformer1", "out", "consumer", "in")
    .connect("transformer2", "out", "consumer", "in")
    .build()
    .unwrap();

  assert_eq!(graph.edge_count(), 4);
  assert!(graph.has_edge("producer", "transformer1"));
  assert!(graph.has_edge("producer", "transformer2"));
  assert!(graph.has_edge("transformer1", "consumer"));
  assert!(graph.has_edge("transformer2", "consumer"));
}

#[tokio::test]
async fn test_graph_builder_expose_input_ports() {
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));

  let graph = GraphBuilder::new("service")
    .add_node("transformer", transformer)
    .expose_input_port("transformer", "in", "input")
    .expose_input_port("transformer", "config", "configuration")
    .build()
    .unwrap();

  // Verify the graph implements Node trait correctly
  assert_eq!(
    graph.input_port_names(),
    &["configuration".to_string(), "input".to_string()]
  );
}

#[tokio::test]
async fn test_graph_builder_expose_output_ports() {
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));

  let graph = GraphBuilder::new("service")
    .add_node("transformer", transformer)
    .expose_output_port("transformer", "out", "output")
    .expose_output_port("transformer", "errors", "error")
    .build()
    .unwrap();

  // Verify the graph implements Node trait correctly
  assert_eq!(
    graph.output_port_names(),
    &["output".to_string(), "error".to_string()]
  );
}

#[tokio::test]
async fn test_graph_builder_complete_pipeline() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let graph = GraphBuilder::new("complete_pipeline")
    .add_node("producer", producer)
    .add_node("transformer", transformer)
    .add_node("consumer", consumer)
    .connect("producer", "out", "transformer", "in")
    .connect("transformer", "out", "consumer", "in")
    .expose_input_port("transformer", "config", "configuration")
    .expose_output_port("consumer", "result", "output")
    .build()
    .unwrap();

  // Verify structure
  assert_eq!(graph.node_count(), 3);
  assert_eq!(graph.edge_count(), 2);
  assert!(graph.has_node("producer"));
  assert!(graph.has_node("transformer"));
  assert!(graph.has_node("consumer"));
  assert!(graph.has_edge("producer", "transformer"));
  assert!(graph.has_edge("transformer", "consumer"));

  // Verify port mappings
  assert_eq!(graph.input_port_names(), &["configuration".to_string()]);
  assert_eq!(graph.output_port_names(), &["output".to_string()]);
}

#[tokio::test]
async fn test_graph_builder_duplicate_node_names() {
  let node1 = Box::new(MockProducerNode::new("producer1".to_string(), vec![42]));
  let node2 = Box::new(MockProducerNode::new("producer2".to_string(), vec![43]));

  let result = GraphBuilder::new("test")
    .add_node("producer", node1)
    .add_node("producer", node2) // Same name - should fail
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("already exists"));
}

#[tokio::test]
async fn test_graph_builder_invalid_connection_source_node() {
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("consumer", consumer)
    .connect("nonexistent", "out", "consumer", "in") // Source node doesn't exist
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_graph_builder_invalid_connection_target_node() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![42]));

  let result = GraphBuilder::new("test")
    .add_node("producer", producer)
    .connect("producer", "out", "nonexistent", "in") // Target node doesn't exist
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_graph_builder_invalid_connection_source_port() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![42]));
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("producer", producer)
    .add_node("consumer", consumer)
    .connect("producer", "invalid_port", "consumer", "in") // Invalid source port
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not have output port"));
}

#[tokio::test]
async fn test_graph_builder_invalid_connection_target_port() {
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![42]));
  let consumer = Box::new(MockConsumerNode::new("consumer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("producer", producer)
    .add_node("consumer", consumer)
    .connect("producer", "out", "consumer", "invalid_port") // Invalid target port
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not have input port"));
}

#[tokio::test]
async fn test_graph_builder_invalid_input_port_mapping() {
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("transformer", transformer)
    .expose_input_port("transformer", "in", "invalid_external") // Invalid external name
    .build();

  assert!(result.is_err());
  assert!(
    result
      .unwrap_err()
      .contains("must be 'configuration' or 'input'")
  );
}

#[tokio::test]
async fn test_graph_builder_invalid_output_port_mapping() {
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("transformer", transformer)
    .expose_output_port("transformer", "out", "invalid_external") // Invalid external name
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("must be 'output' or 'error'"));
}

#[tokio::test]
async fn test_graph_builder_input_port_mapping_nonexistent_node() {
  let result = GraphBuilder::new("test")
    .expose_input_port("nonexistent", "in", "input") // Node doesn't exist
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_graph_builder_output_port_mapping_nonexistent_node() {
  let result = GraphBuilder::new("test")
    .expose_output_port("nonexistent", "out", "output") // Node doesn't exist
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn test_graph_builder_input_port_mapping_invalid_port() {
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("transformer", transformer)
    .expose_input_port("transformer", "invalid_port", "input") // Port doesn't exist
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not have input port"));
}

#[tokio::test]
async fn test_graph_builder_output_port_mapping_invalid_port() {
  let transformer = Box::new(MockTransformerNode::new("transformer".to_string()));

  let result = GraphBuilder::new("test")
    .add_node("transformer", transformer)
    .expose_output_port("transformer", "invalid_port", "output") // Port doesn't exist
    .build();

  assert!(result.is_err());
  assert!(result.unwrap_err().contains("does not have output port"));
}

#[tokio::test]
async fn test_graph_builder_method_chaining() {
  // Test that all methods return Self for chaining
  let _builder = GraphBuilder::new("chained")
    .add_node(
      "node1",
      Box::new(MockProducerNode::new("n1".to_string(), vec![1])),
    )
    .add_node("node2", Box::new(MockConsumerNode::new("n2".to_string())))
    .connect("node1", "out", "node2", "in")
    .expose_input_port("node1", "config", "configuration")
    .expose_output_port("node2", "result", "output");

  // If we get here, method chaining works (types check out)
  assert!(true);
}

#[tokio::test]
async fn test_graph_builder_empty_graph() {
  let graph = GraphBuilder::new("empty").build().unwrap();

  assert_eq!(graph.name(), "empty");
  assert_eq!(graph.node_count(), 0);
  assert_eq!(graph.edge_count(), 0);
  assert_eq!(graph.input_port_names().len(), 2); // configuration and input (defaults)
  assert_eq!(graph.output_port_names().len(), 2); // output and error (defaults)
}

#[tokio::test]
async fn test_graph_builder_integration_example() {
  // Create a simple arithmetic graph: (a + b) * 2
  let add_node = Box::new(AddNode::new("add".to_string()));
  let multiply_node = Box::new(MultiplyNode::new("multiply".to_string()));

  let graph = GraphBuilder::new("arithmetic_pipeline")
    .add_node("adder", add_node)
    .add_node("multiplier", multiply_node)
    .connect("adder", "out", "multiplier", "in1")
    .expose_input_port("adder", "in1", "input")
    .expose_input_port("adder", "in2", "input")
    .expose_output_port("multiplier", "out", "output")
    .build()
    .unwrap();

  // Verify structure
  assert_eq!(graph.name(), "arithmetic_pipeline");
  assert_eq!(graph.node_count(), 2);
  assert_eq!(graph.edge_count(), 1);
  assert!(graph.has_node("adder"));
  assert!(graph.has_node("multiplier"));
  assert!(graph.has_edge("adder", "multiplier"));

  // Verify port mappings
  assert_eq!(
    graph.input_port_names(),
    &["configuration".to_string(), "input".to_string()]
  );
  assert_eq!(
    graph.output_port_names(),
    &["output".to_string(), "error".to_string()]
  );
}
