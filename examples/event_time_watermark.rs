//! # Event-time progress and watermarks
//!
//! Demonstrates [`execute_with_progress`], [`WatermarkInjectorNode`], and frontier observation.
//!
//! - Builds a graph: producer → WatermarkInjectorNode → exposed output.
//! - Runs with [`Graph::execute_with_progress`] to get a [`ProgressHandle`].
//! - As items reach the sink, the completed frontier advances; we observe it via
//!   `progress.frontier()` and `progress.less_than(t)`.
//!
//! See [docs/progress-tracking.md](docs/progress-tracking.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::stream::WatermarkInjectorNode;
use streamweave::time::{LogicalTime, StreamMessage};
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

/// Producer that emits a few payloads with event_timestamp for the watermark node.
struct ProducerNode {
  name: String,
  events: Vec<(u64, String)>,
  output_port_names: Vec<String>,
}

impl ProducerNode {
  fn new(name: String, events: Vec<(u64, String)>) -> Self {
    Self {
      name,
      events,
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
    let events = self.events.clone();
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(10);
      tokio::spawn(async move {
        for (event_time_ms, label) in events {
          let mut map = HashMap::new();
          map.insert(
            "event_timestamp".to_string(),
            Arc::new(event_time_ms) as Arc<dyn Any + Send + Sync>,
          );
          map.insert(
            "label".to_string(),
            Arc::new(label) as Arc<dyn Any + Send + Sync>,
          );
          let _ = tx.send(Arc::new(map) as Arc<dyn Any + Send + Sync>).await;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!("Event-time progress and watermarks example");
  println!("  execute_with_progress, WatermarkInjectorNode, frontier observation\n");

  let mut graph = Graph::new("event_time_watermark".to_string());

  let producer = Box::new(ProducerNode::new(
    "producer".to_string(),
    vec![
      (100, "first".to_string()),
      (200, "second".to_string()),
      (300, "third".to_string()),
    ],
  ));
  let watermark = Box::new(WatermarkInjectorNode::new("watermark".to_string()));

  graph.add_node("producer".to_string(), producer).unwrap();
  graph.add_node("watermark".to_string(), watermark).unwrap();
  graph
    .add_edge(Edge {
      source_node: "producer".to_string(),
      source_port: "out".to_string(),
      target_node: "watermark".to_string(),
      target_port: "in".to_string(),
    })
    .unwrap();
  graph
    .expose_output_port("watermark", "out", "output")
    .unwrap();

  let (tx, mut rx) = mpsc::channel(10);
  graph.connect_output_channel("output", tx).unwrap();

  let progress = graph.execute_with_progress().await?;
  println!(
    "Graph running with progress tracking. Initial frontier: {:?}",
    progress.frontier()
  );

  let mut received = 0usize;
  while let Some(arc) = rx.recv().await {
    if let Ok(msg) = arc.downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>() {
      match msg.as_ref() {
        StreamMessage::Data(ts) => {
          received += 1;
          println!(
            "  Received data at time {:?}, frontier now {:?}",
            ts.time(),
            progress.frontier()
          );
        }
        StreamMessage::Watermark(t) => {
          println!(
            "  Received watermark {:?}, frontier now {:?}",
            t,
            progress.frontier()
          );
        }
      }
    }
  }

  println!("\nReceived {} data items.", received);
  println!(
    "Final frontier: {:?} (less_than(LogicalTime(301)) = {})",
    progress.frontier(),
    progress.less_than(LogicalTime::new(301))
  );

  graph.wait_for_completion().await?;
  println!("Done.");
  Ok(())
}
