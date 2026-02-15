//! Integration tests for time-range recomputation.

use crate::edge::Edge;
use crate::graph::Graph;
use crate::incremental::{RecomputePlan, RecomputeRequest, TimeRange};
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::time::LogicalTime;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Minimal producer node for tests (0 inputs, 1 output).
struct TestProducerNode {
  name: String,
  data: Vec<i32>,
  output_ports: Vec<String>,
}

impl TestProducerNode {
  fn new(name: impl Into<String>, data: Vec<i32>) -> Self {
    Self {
      name: name.into(),
      data,
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for TestProducerNode {
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
    &self.output_ports
  }
  fn has_input_port(&self, _: &str) -> bool {
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
        Box::pin(ReceiverStream::new(rx)) as crate::node::InputStream,
      );
      Ok(outputs)
    })
  }
}

#[tokio::test]
async fn graph_plan_recompute_returns_source_when_single_node() {
  let mut graph = Graph::new("g".to_string());
  graph
    .add_node(
      "a".to_string(),
      Box::new(TestProducerNode::new("a", vec![1, 2, 3])),
    )
    .unwrap();
  graph.expose_output_port("a", "out", "output").unwrap();

  let req = RecomputeRequest::new(
    TimeRange::new(LogicalTime::new(1), LogicalTime::new(5)),
    Some("a".into()),
  );
  let plan = graph.plan_recompute(&req, None);
  assert_eq!(plan.nodes, vec!["a"]);
}

#[tokio::test]
async fn graph_plan_recompute_source_and_downstream() {
  use crate::nodes::map_node::MapNode;

  let mut graph = Graph::new("g".to_string());
  graph
    .add_node(
      "a".to_string(),
      Box::new(TestProducerNode::new("a", vec![1])),
    )
    .unwrap();
  graph
    .add_node("b".to_string(), Box::new(MapNode::new("b".to_string())))
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph.expose_output_port("b", "out", "output").unwrap();

  let req = RecomputeRequest::new(
    TimeRange::new(LogicalTime::new(1), LogicalTime::new(5)),
    Some("a".into()),
  );
  let plan = graph.plan_recompute(&req, None);
  assert_eq!(plan.nodes, vec!["a", "b"]);
}

#[tokio::test]
async fn graph_execute_recompute_runs_full_graph() {
  let mut graph = Graph::new("g".to_string());
  graph
    .add_node(
      "producer".to_string(),
      Box::new(TestProducerNode::new("producer", vec![1, 2, 3])),
    )
    .unwrap();
  graph
    .expose_output_port("producer", "out", "output")
    .unwrap();

  let (tx, mut rx) = tokio::sync::mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  let req = RecomputeRequest::new(
    TimeRange::new(LogicalTime::new(0), LogicalTime::new(10)),
    Some("producer".into()),
  );
  let plan = graph.plan_recompute(&req, None);
  assert_eq!(plan.nodes, vec!["producer"]);

  graph.execute_recompute(&plan).await.unwrap();
  graph.wait_for_completion().await.unwrap();

  let mut count = 0;
  while rx.recv().await.is_some() {
    count += 1;
  }
  assert_eq!(count, 3);
}

#[tokio::test]
async fn graph_execute_for_time_range_runs_when_plan_equals_all_nodes() {
  let mut graph = Graph::new("g".to_string());
  graph
    .add_node(
      "producer".to_string(),
      Box::new(TestProducerNode::new("producer", vec![1, 2])),
    )
    .unwrap();
  graph
    .expose_output_port("producer", "out", "output")
    .unwrap();

  let (tx, mut rx) = tokio::sync::mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  let req = RecomputeRequest::new(
    TimeRange::new(LogicalTime::new(0), LogicalTime::new(5)),
    Some("producer".into()),
  );
  let plan = graph.plan_recompute(&req, None);
  let time_range = req.time_range;

  graph
    .execute_for_time_range(&plan, time_range)
    .await
    .unwrap();
  graph.wait_for_completion().await.unwrap();

  let mut count = 0;
  while rx.recv().await.is_some() {
    count += 1;
  }
  assert_eq!(count, 2);
}

#[tokio::test]
async fn graph_execute_for_time_range_empty_plan_returns_ok() {
  let mut graph = Graph::new("g".to_string());
  let plan = RecomputePlan { nodes: vec![] };
  let time_range = TimeRange::new(LogicalTime::new(0), LogicalTime::new(10));
  graph
    .execute_for_time_range(&plan, time_range)
    .await
    .unwrap();
}
