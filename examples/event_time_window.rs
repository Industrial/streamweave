//! # Event-time windows (tumbling, watermarks, late-data policy)
//!
//! Demonstrates event-time windowing with watermarks and late-data policy.
//! Graph is loaded from [event_time_window.mmd](event_time_window.mmd).
//!
//! See [docs/windowing.md](docs/windowing.md) and [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

mod run {
  use async_trait::async_trait;
  use std::any::Any;
  use std::collections::HashMap;
  use std::path::Path;
  use std::pin::Pin;
  use std::sync::Arc;
  use std::time::Duration;
  use streamweave::mermaid::{
  blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint, NodeRegistry,
};
  use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
  use streamweave::nodes::stream::{
    LateDataPolicy, TumblingEventTimeWindowNode, WatermarkInjectorNode,
  };
  use tokio::sync::mpsc;
  use tokio_stream::{Stream, wrappers::ReceiverStream};

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

  pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Event-time window example (tumbling, watermarks, late-data policy)\n");

    let mmd_path = Path::new("examples/event_time_window.mmd");
    let bp = parse_mmd_file_to_blueprint(mmd_path)?;
    let mut registry = NodeRegistry::new();
    registry.register("Producer", |id, _inputs, _outputs| {
      Box::new(ProducerNode::new(
        id,
        vec![
          (100, "a".to_string()),
          (500, "b".to_string()),
          (1100, "c".to_string()),
          (1500, "d".to_string()),
        ],
      ))
    });
    registry.register("WatermarkInjectorNode", |id, _inputs, _outputs| {
      Box::new(WatermarkInjectorNode::new(id))
    });
    registry.register("TumblingEventTimeWindowNode", |id, _inputs, _outputs| {
      Box::new(
        TumblingEventTimeWindowNode::new(id, Duration::from_millis(1000))
          .with_late_data_policy(LateDataPolicy::SideOutput),
      )
    });
    let mut graph = blueprint_to_graph(&bp, Some(&registry))?;

    let (tx, mut rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
    graph.connect_output_channel("output", tx).unwrap();

    let _progress = graph.execute_with_progress().await?;

    let mut window_count = 0usize;
    while let Some(arc) = rx.recv().await {
      if let Ok(v) = arc.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        window_count += 1;
        println!("Window {}: {} items", window_count, v.len());
        for (i, item) in v.iter().enumerate() {
          if let Ok(m) = item
            .clone()
            .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
          {
            let label = m
              .get("label")
              .and_then(|a| a.clone().downcast::<String>().ok())
              .map(|s| s.as_ref().clone())
              .unwrap_or_else(|| "?".to_string());
            println!("  [{}] label={}", i, label);
          }
        }
      }
    }

    println!("\nTotal windows received: {}", window_count);
    graph.wait_for_completion().await?;
    println!("Done.");
    Ok(())
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  run::run().await
}
