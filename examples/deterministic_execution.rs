//! # Deterministic execution
//!
//! Demonstrates reproducible runs with [`execute_deterministic`] and
//! [`MergeNode::new_deterministic`]. Same graph run twice yields identical output order.
//!
//! Graph is loaded from [deterministic_execution.mmd](deterministic_execution.mmd).
//!
//! See [docs/deterministic-execution.md](docs/deterministic-execution.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

mod run {
  use async_trait::async_trait;
  use std::any::Any;
  use std::collections::HashMap;
  use std::path::Path;
  use std::pin::Pin;
  use std::sync::Arc;
  use streamweave::mermaid::{
  blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint, NodeRegistry,
};
  use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
  use streamweave::nodes::stream::MergeNode;
  use tokio::sync::mpsc;
  use tokio_stream::{Stream, wrappers::ReceiverStream};

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

  async fn run_once() -> Result<Vec<i32>, Box<dyn std::error::Error + Send + Sync>> {
    let mmd_path = Path::new("examples/deterministic_execution.mmd");
    let bp = parse_mmd_file_to_blueprint(mmd_path)?;
    let mut registry = NodeRegistry::new();
    registry.register("Producer", |id, _inputs, _outputs| {
      let data = if id == "a" {
        vec![1, 2, 3]
      } else {
        vec![10, 20]
      };
      Box::new(ProducerNode::new(id, data))
    });
    registry.register("MergeNode", |id, inputs, _outputs| {
      Box::new(MergeNode::new_deterministic(id, inputs.len()))
    });
    let mut graph = blueprint_to_graph(&bp, Some(&registry))?;

    let (tx, mut rx) = mpsc::channel(10);
    graph.connect_output_channel("output", tx).unwrap();

    graph.execute_deterministic().await?;

    let mut out = Vec::new();
    while let Some(arc) = rx.recv().await {
      if let Ok(x) = arc.downcast::<i32>() {
        out.push(*x);
      }
    }
    graph.wait_for_completion().await?;
    Ok(out)
  }

  pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
      "Deterministic execution example (execute_deterministic, MergeNode::new_deterministic)\n"
    );

    let run1 = run_once().await?;
    let run2 = run_once().await?;

    println!("Run 1: {:?}", run1);
    println!("Run 2: {:?}", run2);
    assert_eq!(
      run1, run2,
      "deterministic mode must produce identical outputs"
    );
    println!("\nBoth runs identical. Done.");
    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  run::run().await
}
