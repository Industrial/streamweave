//! # Exactly-once state
//!
//! Demonstrates [`KeyedStateBackend`], [`StatefulNodeDriver`], and idempotent
//! semantics: keyed state with logical-time versioning, snapshot/restore, and
//! an idempotent sink-style write pattern.
//!
//! The example also runs a graph loaded from [exactly_once_state.mmd](exactly_once_state.mmd)
//! (Style B Mermaid diagram) so that all examples use a graph.
//!
//! See [docs/exactly-once-state.md](docs/exactly-once-state.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use std::any::Any;
use std::path::Path;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::mermaid::{
  NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
};
use streamweave::state::{
  ExactlyOnceStateBackend, HashMapStateBackend, KeyedStateBackend, StatefulNodeDriver,
};
use streamweave::time::LogicalTime;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

mod run {
  use async_trait::async_trait;
  use std::any::Any;
  use std::collections::HashMap;
  use std::pin::Pin;
  use std::sync::Arc;
  use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
  use tokio::sync::mpsc;
  use tokio_stream::{Stream, wrappers::ReceiverStream};

  /// Minimal stateful node that emits one item so the graph runs.
  pub struct StatefulNode {
    name: String,
    output_port_names: Vec<String>,
  }

  impl StatefulNode {
    pub fn new(name: String) -> Self {
      Self {
        name,
        output_port_names: vec!["out".to_string()],
      }
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
    fn execute(
      &self,
      _inputs: InputStreams,
    ) -> Pin<
      Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
    > {
      Box::pin(async move {
        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
          let _ = tx.send(Arc::new("ok") as Arc<dyn Any + Send + Sync>).await;
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!(
    "Exactly-once state example (KeyedStateBackend, StatefulNodeDriver, snapshot/restore)\n"
  );

  // KeyedStateBackend is HashMapStateBackend<K, V, LogicalTime>
  let backend: KeyedStateBackend<String, u64> = HashMapStateBackend::new();

  // --- StatefulNodeDriver: apply_update with LogicalTime (idempotent) ---
  println!("1. Applying updates with logical times (StatefulNodeDriver)");
  backend.apply_update("user:alice".to_string(), 100u64, LogicalTime::new(1))?;
  backend.apply_update("user:bob".to_string(), 200u64, LogicalTime::new(1))?;
  backend.apply_update("user:alice".to_string(), 150u64, LogicalTime::new(2))?;

  assert_eq!(
    backend.get_value(&"user:alice".to_string())?,
    Some((150u64, LogicalTime::new(2)))
  );
  assert_eq!(
    backend.get_value(&"user:bob".to_string())?,
    Some((200u64, LogicalTime::new(1)))
  );
  println!("   user:alice -> 150 @ t=2, user:bob -> 200 @ t=1");

  // --- Idempotency: same version applied again does not change state ---
  println!("\n2. Idempotent put (same version again is a no-op)");
  backend.apply_update("user:alice".to_string(), 999u64, LogicalTime::new(2))?;
  assert_eq!(
    backend.get_value(&"user:alice".to_string())?,
    Some((150u64, LogicalTime::new(2))),
    "re-applying at t=2 must not overwrite"
  );
  println!("   Re-applied (alice, 999, t=2) -> state unchanged (150 @ t=2)");

  // --- Snapshot and restore ---
  println!("\n3. Snapshot and restore");
  let bytes = backend.snapshot_bytes()?;
  println!("   Snapshot size: {} bytes", bytes.len());

  let mut restored: KeyedStateBackend<String, u64> = HashMapStateBackend::new();
  restored.restore(&bytes)?;
  assert_eq!(
    restored.get_value(&"user:alice".to_string())?,
    Some((150u64, LogicalTime::new(2)))
  );
  assert_eq!(
    restored.get_value(&"user:bob".to_string())?,
    Some((200u64, LogicalTime::new(1)))
  );
  println!("   Restored backend has same state as original");

  // --- Idempotent sink pattern ---
  println!("\n4. Idempotent sink pattern (stable key + version per record)");
  let sink_state: KeyedStateBackend<String, u64> = HashMapStateBackend::new();
  for (key, value, time) in [
    ("rec:1".to_string(), 10u64, LogicalTime::new(1)),
    ("rec:2".to_string(), 20u64, LogicalTime::new(2)),
    ("rec:1".to_string(), 10u64, LogicalTime::new(1)),
  ] {
    sink_state.apply_update(key.clone(), value, time)?;
  }
  assert_eq!(
    sink_state.get_value(&"rec:1".to_string())?,
    Some((10u64, LogicalTime::new(1)))
  );
  println!("   Duplicate (rec:1, 10, t=1) did not double-apply");

  // --- Run graph from .mmd (all examples use a graph) ---
  println!("\n5. Running graph from exactly_once_state.mmd");
  let path = Path::new("examples/exactly_once_state.mmd");
  let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
  let mut registry = NodeRegistry::new();
  registry.register("StatefulNode", |id, _inputs, _outputs| {
    Box::new(run::StatefulNode::new(id))
  });
  let mut graph = blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?;
  let (tx, mut rx) = mpsc::channel(10);
  graph.connect_output_channel("output", tx)?;
  graph.execute().await.map_err(|e| format!("{:?}", e))?;
  let _ = rx.recv().await;
  graph.wait_for_completion().await?;
  println!("   Graph ran successfully.");

  println!("\nDone.");
  Ok(())
}
