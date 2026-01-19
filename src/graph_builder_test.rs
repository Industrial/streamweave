//! # GraphBuilder Test Suite
//!
//! Comprehensive test suite for the `GraphBuilder` struct, covering fluent API construction,
//! validation, and integration with the Graph type.

use crate::graph_builder::GraphBuilder;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::arithmetic::{AddNode, MultiplyNode};
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
struct MockTransformerNode {
  name: String,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
}

impl MockTransformerNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for MockTransformerNode {
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

/// Mock consumer node (1 input, 0 outputs) - alias for MockSinkNode
type MockConsumerNode = MockSinkNode;

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
