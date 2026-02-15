//! # Checkpoint and restore
//!
//! Demonstrates [`trigger_checkpoint`], [`restore_from_checkpoint`], and
//! [`FileCheckpointStorage`]. A stateful node snapshots its state; we checkpoint,
//! then restore into a new graph and show [`restored_position`].
//!
//! See [docs/distributed-checkpointing.md](docs/distributed-checkpointing.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use streamweave::checkpoint::{CheckpointStorage, FileCheckpointStorage};
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::{Stream, wrappers::ReceiverStream};

/// Minimal stateful node: holds bytes for snapshot/restore.
struct StatefulNode {
  name: String,
  state: Mutex<Vec<u8>>,
  output_port_names: Vec<String>,
}

impl StatefulNode {
  fn new(name: String, initial_state: Vec<u8>) -> Self {
    Self {
      name,
      state: Mutex::new(initial_state),
      output_port_names: vec!["out".to_string()],
    }
  }

  #[allow(dead_code)]
  fn state(&self) -> Vec<u8> {
    self.state.lock().unwrap().clone()
  }
}

#[async_trait]
impl Node for StatefulNode {
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
  fn snapshot_state(&self) -> Result<Vec<u8>, NodeExecutionError> {
    Ok(self.state.lock().unwrap().clone())
  }
  fn restore_state(&mut self, data: &[u8]) -> Result<(), NodeExecutionError> {
    *self.state.lock().unwrap() = data.to_vec();
    Ok(())
  }
  fn execute(
    &self,
    _inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(10);
      drop(tx);
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

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!("Checkpoint and restore example\n");

  let base = std::env::temp_dir().join("streamweave_checkpoint_example");
  std::fs::create_dir_all(&base).map_err(|e: io::Error| e.to_string())?;
  let storage = FileCheckpointStorage::new(&base);

  // Build graph with stateful node
  let mut graph = Graph::new("checkpoint_demo".to_string());
  let node = StatefulNode::new("stateful".to_string(), b"hello-checkpoint".to_vec());
  graph
    .add_node("stateful".to_string(), Box::new(node))
    .unwrap();

  // Checkpoint
  let id = graph.trigger_checkpoint(&storage)?;
  println!("Triggered checkpoint: id = {}", id.as_u64());

  let ids = storage.list()?;
  println!(
    "List checkpoints: {:?}",
    ids.iter().map(|id| id.as_u64()).collect::<Vec<_>>()
  );

  // Restore into a new graph (simulating restart)
  let mut graph2 = Graph::new("restored".to_string());
  graph2
    .add_node(
      "stateful".to_string(),
      Box::new(StatefulNode::new("stateful".to_string(), vec![])),
    )
    .unwrap();
  graph2.restore_from_checkpoint(&storage, id)?;
  println!("Restored position: {:?}", graph2.restored_position());

  // Verify state was restored
  let nodes = graph2.get_nodes();
  let stateful = nodes.get("stateful").unwrap();
  let snapshot = stateful.snapshot_state().unwrap();
  let state_str = String::from_utf8_lossy(&snapshot);
  println!("Restored node state: {}", state_str);
  drop(nodes);
  assert_eq!(snapshot, b"hello-checkpoint");

  println!("\nDone.");
  Ok(())
}
