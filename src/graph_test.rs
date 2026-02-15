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

use crate::checkpoint::{
  CheckpointId, CheckpointMetadata, CheckpointStorage, FileCheckpointStorage,
};
use crate::edge::Edge;
use crate::graph::{Graph, topological_sort};
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::supervision::{FailureAction, SupervisionPolicy};
use crate::time::LogicalTime;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
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

/// Node that fails the first N times, then succeeds. Uses a shared counter so
/// state persists across graph restarts (execute_with_supervision).
struct FailNTimesThenSucceedNode {
  name: String,
  count: Arc<AtomicU32>,
  fail_count: u32,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
}

impl FailNTimesThenSucceedNode {
  fn new(name: String, fail_count: u32) -> Self {
    Self {
      name,
      count: Arc::new(AtomicU32::new(0)),
      fail_count,
      input_port_names: vec!["in".to_string()],
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for FailNTimesThenSucceedNode {
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
    let count = Arc::clone(&self.count);
    let fail_count = self.fail_count;
    Box::pin(async move {
      let c = count.fetch_add(1, Ordering::SeqCst);
      if c < fail_count {
        return Err(format!("Failing attempt {} (fail_count={})", c + 1, fail_count).into());
      }
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let output_stream: OutputStreams = {
        let mut map = HashMap::new();
        map.insert(
          "out".to_string(),
          Box::pin(input_stream) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
        map
      };
      Ok(output_stream)
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
// Shard Config Tests
// ============================================================================

#[test]
fn test_subgraph_supervision_unit() {
  let mut graph = Graph::new("test".to_string());
  assert!(!graph.is_subgraph_supervision_unit("sub"));
  graph.set_subgraph_supervision_unit("sub");
  assert!(graph.is_subgraph_supervision_unit("sub"));
}

#[test]
fn test_shard_config() {
  let mut graph = Graph::new("test".to_string());
  assert!(graph.shard_id().is_none());
  assert!(graph.total_shards().is_none());
  assert!(graph.shard_config().is_none());

  graph.set_shard_config(1, 4);
  assert_eq!(graph.shard_id(), Some(1));
  assert_eq!(graph.total_shards(), Some(4));
  let config = graph.shard_config().unwrap();
  assert_eq!(config.shard_id, 1);
  assert_eq!(config.total_shards, 4);
  // owns_key is deterministic: same key always hashes to same shard
  assert_eq!(config.owns_key("user_42"), config.owns_key("user_42"));
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
// Cycle Detection Tests (Option B - Timely-style rounds)
// ============================================================================

#[test]
fn test_has_cycles_acyclic() {
  let mut graph = Graph::new("test".to_string());
  let a = Box::new(MockProducerNode::new("a".to_string(), vec![1]));
  let b = Box::new(MockTransformNode::new("b".to_string()));
  let c = Box::new(MockSinkNode::new("c".to_string()));
  graph.add_node("a".to_string(), a).unwrap();
  graph.add_node("b".to_string(), b).unwrap();
  graph.add_node("c".to_string(), c).unwrap();
  graph
    .add_edge(Edge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "b".to_string(),
      source_port: "out".to_string(),
      target_node: "c".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  assert!(!graph.has_cycles());
  assert!(graph.feedback_edges().is_empty());
}

#[test]
fn test_has_cycles_cyclic() {
  let mut graph = Graph::new("test".to_string());
  let a = Box::new(MockTransformNode::new("a".to_string()));
  let b = Box::new(MockTransformNode::new("b".to_string()));
  graph.add_node("a".to_string(), a).unwrap();
  graph.add_node("b".to_string(), b).unwrap();
  graph
    .add_edge(Edge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "b".to_string(),
      source_port: "out".to_string(),
      target_node: "a".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  assert!(graph.has_cycles());
  let feedback = graph.feedback_edges();
  assert_eq!(feedback.len(), 1);
  assert_eq!(feedback[0].source_node(), "b");
  assert_eq!(feedback[0].target_node(), "a");
}

#[test]
fn test_nodes_depending_on_and_downstream() {
  let mut graph = Graph::new("test".to_string());
  let a = Box::new(MockProducerNode::new("a".to_string(), vec![1]));
  let b = Box::new(MockTransformNode::new("b".to_string()));
  let c = Box::new(MockTransformNode::new("c".to_string()));
  let d = Box::new(MockSinkNode::new("d".to_string()));
  graph.add_node("a".to_string(), a).unwrap();
  graph.add_node("b".to_string(), b).unwrap();
  graph.add_node("c".to_string(), c).unwrap();
  graph.add_node("d".to_string(), d).unwrap();
  graph
    .add_edge(Edge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "c".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "b".to_string(),
      source_port: "out".to_string(),
      target_node: "d".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "c".to_string(),
      source_port: "out".to_string(),
      target_node: "d".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  let deps_a = graph.nodes_depending_on("a");
  assert_eq!(deps_a.len(), 2);
  assert!(deps_a.contains(&"b".to_string()));
  assert!(deps_a.contains(&"c".to_string()));
  let downstream_a = graph.nodes_downstream_transitive("a");
  assert_eq!(downstream_a.len(), 3);
  assert!(downstream_a.contains(&"b".to_string()));
  assert!(downstream_a.contains(&"c".to_string()));
  assert!(downstream_a.contains(&"d".to_string()));
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

  // Execute the graph (use Graph::execute to disambiguate from Node::execute)
  assert!(Graph::execute(&mut graph).await.is_ok());

  // Wait for completion
  assert!(graph.wait_for_completion().await.is_ok());
}

#[tokio::test]
async fn test_is_ready_and_is_live() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
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

  assert!(!graph.is_ready());
  assert!(graph.is_live());

  Graph::execute(&mut graph).await.unwrap();
  assert!(graph.is_ready());

  graph.wait_for_completion().await.unwrap();
  assert!(!graph.is_ready());
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

  // Execute the graph (use Graph::execute to disambiguate from Node::execute)
  assert!(Graph::execute(&mut graph).await.is_ok());

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

  Graph::execute(&mut graph).await.unwrap();

  // Stop execution
  assert!(graph.stop().await.is_ok());

  // Wait for completion (should complete quickly after stop)
  assert!(graph.wait_for_completion().await.is_ok());
}

#[tokio::test]
async fn test_execute_with_supervision_restart_on_failure() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let fail_node = Box::new(FailNTimesThenSucceedNode::new("fail_node".to_string(), 1));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("fail_node".to_string(), fail_node).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "fail_node".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "fail_node".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph.set_node_supervision_policy(
    "fail_node",
    SupervisionPolicy::new(FailureAction::Restart).with_max_restarts(Some(2)),
  );

  graph.execute_with_supervision(|_| Ok(())).await.unwrap();
}

#[tokio::test]
async fn test_execute_with_supervision_stop_on_failure() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  let fail_node = Box::new(FailNTimesThenSucceedNode::new("fail_node".to_string(), 10));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("fail_node".to_string(), fail_node).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "fail_node".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "fail_node".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph.set_node_supervision_policy(
    "fail_node",
    SupervisionPolicy::new(FailureAction::Stop).with_max_restarts(Some(0)),
  );

  let err = graph
    .execute_with_supervision(|_| Ok(()))
    .await
    .unwrap_err();
  assert!(err.to_string().contains("Failing attempt"));
}

#[tokio::test]
async fn test_execute_with_supervision_restart_group() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2]));
  let fail_node = Box::new(FailNTimesThenSucceedNode::new("fail_node".to_string(), 1));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("fail_node".to_string(), fail_node).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "fail_node".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "fail_node".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph.set_node_supervision_policy(
    "fail_node",
    SupervisionPolicy::new(FailureAction::RestartGroup).with_max_restarts(Some(2)),
  );

  graph.execute_with_supervision(|_| Ok(())).await.unwrap();
}

#[tokio::test]
async fn test_execute_with_supervision_escalate() {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  let fail_node = Box::new(FailNTimesThenSucceedNode::new("fail_node".to_string(), 10));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("fail_node".to_string(), fail_node).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "fail_node".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "fail_node".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph.set_node_supervision_policy("fail_node", SupervisionPolicy::new(FailureAction::Escalate));

  let err = graph
    .execute_with_supervision(|_| Ok(()))
    .await
    .unwrap_err();
  assert!(err.to_string().contains("Failing attempt"));
}

#[tokio::test]
async fn test_execute_with_supervision_restart_group_shared_count() {
  // Two nodes in the same group share restart count. A fails first run, B fails second run.
  // With max_restarts=2 for the group, both failures count toward the group limit; third run succeeds.
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2]));
  let fail_a = Box::new(FailNTimesThenSucceedNode::new("fail_a".to_string(), 1));
  let fail_b = Box::new(FailNTimesThenSucceedNode::new("fail_b".to_string(), 1));
  let sink = Box::new(MockSinkNode::new("sink".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("fail_a".to_string(), fail_a).unwrap();
  graph.add_node("fail_b".to_string(), fail_b).unwrap();
  graph.add_node("sink".to_string(), sink).unwrap();

  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "fail_a".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "fail_b".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  let sink_b = Box::new(MockSinkNode::new("sink_b".to_string()));
  graph.add_node("sink_b".to_string(), sink_b).unwrap();
  graph
    .add_edge(Edge {
      source_node: "fail_a".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "fail_b".to_string(),
      source_port: "out".to_string(),
      target_node: "sink_b".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();

  graph.set_supervision_group("fail_a", "group1");
  graph.set_supervision_group("fail_b", "group1");
  graph.set_node_supervision_policy(
    "fail_a",
    SupervisionPolicy::new(FailureAction::RestartGroup).with_max_restarts(Some(2)),
  );
  graph.set_node_supervision_policy(
    "fail_b",
    SupervisionPolicy::new(FailureAction::RestartGroup).with_max_restarts(Some(2)),
  );

  graph.execute_with_supervision(|_| Ok(())).await.unwrap();
}

#[tokio::test]
async fn test_execute_with_progress_advances_frontier() {
  // Pipeline: producer -> transform; expose transform output. With progress tracking,
  // the frontier should advance when items reach the exposed output.
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new(
    "producer".to_string(),
    vec![10, 20, 30],
  ));
  let transform = Box::new(MockTransformNode::new("transform".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("transform".to_string(), transform).unwrap();
  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "transform".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .expose_output_port("transform", "out", "output")
    .unwrap();

  let (tx, mut rx) = mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  let progress = graph.execute_with_progress().await.unwrap();
  // Initially frontier is 0
  assert!(progress.less_than(LogicalTime::new(1)));

  // Consume the three items from the output; each delivery advances the frontier
  let _ = rx.recv().await.unwrap();
  let _ = rx.recv().await.unwrap();
  let _ = rx.recv().await.unwrap();

  // After three items (times 0, 1, 2) reached the sink, frontier should be at least 2
  assert!(!progress.less_than(LogicalTime::new(3)));
  assert!(progress.frontier() >= LogicalTime::new(2));

  assert!(graph.wait_for_completion().await.is_ok());
}

// ============================================================================
// Determinism Tests
// ============================================================================

/// Builds producer->transform graph, runs execute_deterministic, collects output.
async fn run_deterministic_and_collect() -> Vec<i32> {
  let mut graph = Graph::new("test".to_string());
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1, 2, 3]));
  let transform = Box::new(MockTransformNode::new("transform".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("transform".to_string(), transform).unwrap();
  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "transform".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .expose_output_port("transform", "out", "output")
    .unwrap();

  let (tx, mut rx) = mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  graph.execute_deterministic().await.unwrap();

  let mut collected = Vec::new();
  for _ in 0..3 {
    if let Some(item) = rx.recv().await
      && let Ok(arc) = item.downcast::<i32>()
    {
      collected.push(*arc);
    }
  }
  graph.wait_for_completion().await.unwrap();
  collected
}

#[tokio::test]
async fn test_execute_deterministic_reproducible() {
  // Run the same graph twice in deterministic mode; outputs must be identical.
  let run1 = run_deterministic_and_collect().await;
  let run2 = run_deterministic_and_collect().await;
  assert_eq!(
    run1, run2,
    "deterministic mode must produce identical outputs"
  );
  assert_eq!(
    run1,
    vec![2, 4, 6],
    "transform multiplies by 2: 1,2,3 -> 2,4,6"
  );
}

// ============================================================================
// Graph as Node Tests
// ============================================================================

#[test]
fn test_graph_has_input_ports() {
  let mut graph = Graph::new("test".to_string());
  // New graphs have no ports by default
  assert!(!graph.has_input_port("configuration"));
  assert!(!graph.has_input_port("input"));
  assert!(!graph.has_input_port("nonexistent"));

  // Add a node and expose a port
  let transform = Box::new(MockTransformNode::new("transform".to_string()));
  graph.add_node("transform".to_string(), transform).unwrap();
  graph.expose_input_port("transform", "in", "input").unwrap();
  assert!(graph.has_input_port("input"));
  assert!(!graph.has_input_port("configuration"));
}

#[test]
fn test_graph_has_output_ports() {
  let mut graph = Graph::new("test".to_string());
  // New graphs have no ports by default
  assert!(!graph.has_output_port("output"));
  assert!(!graph.has_output_port("error"));
  assert!(!graph.has_output_port("nonexistent"));

  // Add a node and expose a port
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  graph.add_node("producer".to_string(), producer).unwrap();
  graph
    .expose_output_port("producer", "out", "output")
    .unwrap();
  assert!(graph.has_output_port("output"));
  assert!(!graph.has_output_port("error"));
}

#[test]
fn test_graph_input_port_names() {
  let mut graph = Graph::new("test".to_string());
  // New graphs have no ports by default
  let ports = graph.input_port_names();
  assert_eq!(ports.len(), 0);

  // Add a node and expose a port
  let transform = Box::new(MockTransformNode::new("transform".to_string()));
  graph.add_node("transform".to_string(), transform).unwrap();
  graph.expose_input_port("transform", "in", "input").unwrap();
  let ports = graph.input_port_names();
  assert_eq!(ports.len(), 1);
  assert!(ports.contains(&"input".to_string()));

  // Expose another port with a different name
  let transform2 = Box::new(MockTransformNode::new("transform2".to_string()));
  graph
    .add_node("transform2".to_string(), transform2)
    .unwrap();
  graph
    .expose_input_port("transform2", "in", "configuration")
    .unwrap();
  let ports = graph.input_port_names();
  assert_eq!(ports.len(), 2);
  assert!(ports.contains(&"configuration".to_string()));
  assert!(ports.contains(&"input".to_string()));
}

#[test]
fn test_graph_output_port_names() {
  let mut graph = Graph::new("test".to_string());
  // New graphs have no ports by default
  let ports = graph.output_port_names();
  assert_eq!(ports.len(), 0);

  // Add a node and expose a port
  let producer = Box::new(MockProducerNode::new("producer".to_string(), vec![1]));
  graph.add_node("producer".to_string(), producer).unwrap();
  graph
    .expose_output_port("producer", "out", "output")
    .unwrap();
  let ports = graph.output_port_names();
  assert_eq!(ports.len(), 1);
  assert!(ports.contains(&"output".to_string()));

  // Expose another port with a different name (using same node's port twice)
  graph
    .expose_output_port("producer", "out", "error")
    .unwrap();
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
  let _ = subgraph
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
  Graph::execute(&mut graph).await.unwrap();

  // Stop should clear state
  assert!(graph.stop().await.is_ok());

  // After stop, execution handles should be cleared
  // (We verify this by checking wait_for_completion completes quickly)
  assert!(graph.wait_for_completion().await.is_ok());
}

// ============================================================================
// Checkpoint Restore Tests
// ============================================================================

/// Mock node that supports snapshot_state and restore_state for checkpoint tests.
struct MockRestorableNode {
  name: String,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
  snapshot_data: Arc<std::sync::Mutex<Vec<u8>>>,
  restored_data: Arc<std::sync::Mutex<Option<Vec<u8>>>>,
}

impl MockRestorableNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      output_port_names: vec!["out".to_string()],
      snapshot_data: Arc::new(std::sync::Mutex::new(Vec::new())),
      restored_data: Arc::new(std::sync::Mutex::new(None)),
    }
  }

  fn with_snapshot_data(self, data: Vec<u8>) -> Self {
    *self.snapshot_data.lock().unwrap() = data;
    self
  }

  #[allow(dead_code)]
  fn restored_data(&self) -> Option<Vec<u8>> {
    self.restored_data.lock().unwrap().clone()
  }
}

#[async_trait]
impl Node for MockRestorableNode {
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

  fn snapshot_state(&self) -> Result<Vec<u8>, NodeExecutionError> {
    Ok(self.snapshot_data.lock().unwrap().clone())
  }

  fn restore_state(&mut self, data: &[u8]) -> Result<(), NodeExecutionError> {
    *self.restored_data.lock().unwrap() = Some(data.to_vec());
    Ok(())
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let _ = inputs.remove("in").ok_or("Missing 'in' input")?;
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(tokio_stream::empty())
          as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}

#[test]
fn test_restore_from_checkpoint() {
  let tmp = tempfile::TempDir::new().unwrap();
  let storage = FileCheckpointStorage::new(tmp.path());

  let metadata = CheckpointMetadata {
    id: CheckpointId::new(1),
    position: Some(LogicalTime::new(100)),
  };
  let mut snapshots = HashMap::new();
  snapshots.insert("stateful".to_string(), b"restored-state-bytes".to_vec());
  storage.save(&metadata, &snapshots).unwrap();

  let mut graph = Graph::new("test".to_string());
  let stateful = MockRestorableNode::new("stateful".to_string());
  let restored_data = Arc::clone(&stateful.restored_data);
  graph
    .add_node("stateful".to_string(), Box::new(stateful))
    .unwrap();

  graph
    .restore_from_checkpoint(&storage, CheckpointId::new(1))
    .unwrap();

  assert_eq!(graph.restored_position(), Some(LogicalTime::new(100)));
  assert_eq!(
    *restored_data.lock().unwrap(),
    Some(b"restored-state-bytes".to_vec())
  );
}

#[test]
fn test_trigger_checkpoint() {
  let tmp = tempfile::TempDir::new().unwrap();
  let storage = FileCheckpointStorage::new(tmp.path());

  let mut graph = Graph::new("test".to_string());
  let stateful =
    MockRestorableNode::new("stateful".to_string()).with_snapshot_data(b"my-state".to_vec());
  graph
    .add_node("stateful".to_string(), Box::new(stateful))
    .unwrap();

  let id = graph.trigger_checkpoint(&storage).unwrap();
  assert_eq!(id.as_u64(), 0);

  let (meta, snapshots) = storage.load(id).unwrap();
  assert_eq!(meta.id.as_u64(), 0);
  assert_eq!(snapshots.get("stateful"), Some(&b"my-state".to_vec()));

  let id2 = graph.trigger_checkpoint(&storage).unwrap();
  assert_eq!(id2.as_u64(), 1);
}

#[test]
fn test_trigger_checkpoint_no_nodes_fails() {
  let tmp = tempfile::TempDir::new().unwrap();
  let storage = FileCheckpointStorage::new(tmp.path());
  let graph = Graph::new("empty".to_string());
  assert!(graph.trigger_checkpoint(&storage).is_err());
}
