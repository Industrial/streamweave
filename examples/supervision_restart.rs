//! # Supervision and restart
//!
//! Demonstrates [`execute_with_supervision`], [`set_node_supervision_policy`], and
//! restart on failure: a node that fails once then succeeds is restarted and the graph completes.
//!
//! See [docs/actor-supervision-trees.md](docs/actor-supervision-trees.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::supervision::{FailureAction, SupervisionPolicy};
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

/// Producer that emits a few i32 values.
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

/// Node that fails the first N runs, then passes through. Count is shared so restarts see it.
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
        return Err(
          format!(
            "Failing attempt {} (will succeed after {} failures)",
            c + 1,
            fail_count
          )
          .into(),
        );
      }
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(input_stream) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!("Supervision and restart example (execute_with_supervision, failure + restart)\n");

  let mut graph = Graph::new("supervision_demo".to_string());
  graph
    .add_node(
      "producer".to_string(),
      Box::new(ProducerNode::new("producer".to_string(), vec![1, 2, 3])),
    )
    .unwrap();
  graph
    .add_node(
      "flaky".to_string(),
      Box::new(FailNTimesThenSucceedNode::new("flaky".to_string(), 1)),
    )
    .unwrap();
  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "flaky".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph.expose_output_port("flaky", "out", "output").unwrap();

  let (tx, mut rx) = mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  graph.set_node_supervision_policy(
    "flaky",
    SupervisionPolicy::new(FailureAction::Restart).with_max_restarts(Some(2)),
  );

  graph.execute_with_supervision(|_| Ok(())).await?;

  let mut received = Vec::new();
  while let Some(arc) = rx.recv().await {
    if let Ok(x) = arc.downcast::<i32>() {
      received.push(*x);
    }
  }
  graph.wait_for_completion().await?;

  println!("Received: {:?}", received);
  assert_eq!(
    received,
    vec![1, 2, 3],
    "after restart, all items should pass through"
  );
  println!("\nDone.");
  Ok(())
}
