//! # Subgraph Example
//!
//! This example demonstrates how to use Graph as a Node (nested graphs/subgraphs).
//! It shows:
//!
//! 1. Creating a subgraph with internal nodes
//! 2. Exposing internal ports as external ports
//! 3. Using the subgraph as a node in a parent graph
//! 4. Connecting to the subgraph's external ports
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example subgraph-node
//! ```

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::{MapNode, map_config, stream::DropNode};
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;

// Minimal source node
struct SourceNode {
  name: String,
  data: Vec<i32>,
}

#[async_trait]
impl Node for SourceNode {
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
    &[]
  }
  fn has_input_port(&self, _: &str) -> bool {
    false
  }
  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }
  fn execute(
    &self,
    _: InputStreams,
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
      Ok(HashMap::from([(
        "out".to_string(),
        Box::pin(ReceiverStream::new(rx))
          as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      )]))
    })
  }
}

// Config source for MapNode
struct ConfigSource {
  name: String,
  config: streamweave::nodes::MapConfig,
}

#[async_trait]
impl Node for ConfigSource {
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
    &[]
  }
  fn has_input_port(&self, _: &str) -> bool {
    false
  }
  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }
  fn execute(
    &self,
    _: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let config = Arc::clone(&self.config);
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(1);
      let config_wrapped: Arc<streamweave::nodes::MapConfig> = Arc::new(config);
      let _ = tx.send(config_wrapped as Arc<dyn Any + Send + Sync>).await;
      Ok(HashMap::from([(
        "out".to_string(),
        Box::pin(ReceiverStream::new(rx))
          as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      )]))
    })
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Subgraph Example");
  println!("=================");

  // ========================================================================
  // Step 1: Create a subgraph with internal nodes
  // ========================================================================
  println!("\n1. Creating subgraph...");
  let mut subgraph = streamweave::graph::Graph::new("multiply_by_ten".to_string());

  // Create internal nodes for the subgraph
  let internal_map = Box::new(MapNode::new("internal_map".to_string()));
  let internal_config = Box::new(ConfigSource {
    name: "internal_config".to_string(),
    config: map_config(|v| async move {
      if let Ok(arc_i32) = v.downcast::<i32>() {
        Ok(Arc::new(*arc_i32 * 10) as Arc<dyn Any + Send + Sync>)
      } else {
        Err("Expected i32".to_string())
      }
    }),
  });

  // Add nodes to subgraph
  subgraph.add_node("internal_map".to_string(), internal_map)?;
  subgraph.add_node("internal_config".to_string(), internal_config)?;

  // Connect internal nodes
  subgraph.add_edge(Edge {
    source_node: "internal_config".to_string(),
    source_port: "out".to_string(),
    target_node: "internal_map".to_string(),
    target_port: "configuration".to_string(),
  })?;

  // ========================================================================
  // Step 2: Expose internal ports as external ports
  // ========================================================================
  println!("2. Exposing internal ports as external ports...");

  // Create a boundary node to receive external input
  // For this example, we'll create a simple pass-through node
  // In practice, you might use a dedicated boundary node or connect directly

  // Expose internal_map's input as the subgraph's "input" port
  // Note: We need to connect external input to internal_map's "in" port
  // Since MapNode has "in" port, we can expose it directly
  // But we need a way to route external input to it

  // For simplicity, let's create a boundary source node that will receive external input
  // Actually, we can't easily do this without a boundary node pattern
  // Let's use a different approach: create an internal source that we'll connect externally

  // Actually, the cleanest approach is to expose the map's input port
  // But MapNode expects "in" port, so we need to ensure the external "input"
  // maps to the internal "in" port of the map node

  // Since we're using MapNode which has "in" and "configuration" ports,
  // we can expose "in" as "input" - but MapNode's "in" is already an input port
  // So we expose it correctly:
  subgraph.expose_input_port("internal_map", "in", "input")?;
  subgraph.expose_output_port("internal_map", "out", "output")?;

  println!("   - External 'input' -> internal_map 'in'");
  println!("   - External 'output' -> internal_map 'out'");

  // ========================================================================
  // Step 3: Use subgraph as a node in parent graph
  // ========================================================================
  println!("3. Creating parent graph with subgraph as a node...");
  let mut parent_graph = streamweave::graph::Graph::new("parent".to_string());

  // Create source node for parent graph
  let parent_source = Box::new(SourceNode {
    name: "parent_source".to_string(),
    data: vec![1, 2, 3, 4, 5],
  });

  // Convert subgraph to a node
  let subgraph_node: Box<dyn Node> = Box::new(subgraph);

  // Create sink for parent graph
  let parent_sink = Box::new(DropNode::new("parent_sink".to_string()));

  // Add nodes to parent graph
  parent_graph.add_node("parent_source".to_string(), parent_source)?;
  parent_graph.add_node("subgraph".to_string(), subgraph_node)?;
  parent_graph.add_node("parent_sink".to_string(), parent_sink)?;

  // ========================================================================
  // Step 4: Connect to subgraph's external ports
  // ========================================================================
  println!("4. Connecting to subgraph's external ports...");

  // Connect parent source to subgraph's input port
  parent_graph.add_edge(Edge {
    source_node: "parent_source".to_string(),
    source_port: "out".to_string(),
    target_node: "subgraph".to_string(),
    target_port: "input".to_string(), // External port name
  })?;

  // Connect subgraph's output to parent sink
  parent_graph.add_edge(Edge {
    source_node: "subgraph".to_string(),
    source_port: "output".to_string(), // External port name
    target_node: "parent_sink".to_string(),
    target_port: "in".to_string(),
  })?;

  println!("   - parent_source -> subgraph (input)");
  println!("   - subgraph (output) -> parent_sink");

  // ========================================================================
  // Step 5: Execute the parent graph
  // ========================================================================
  println!("\n5. Executing parent graph...");
  println!(
    "   Parent graph: {} nodes, {} edges",
    parent_graph.get_nodes().len(),
    parent_graph.get_edges().len()
  );
  println!("   Processing: [1, 2, 3, 4, 5] -> Subgraph(x * 10) -> Drop");

  parent_graph
    .execute()
    .await
    .map_err(|e| format!("Execution error: {}", e))?;
  parent_graph
    .wait_for_completion()
    .await
    .map_err(|e| format!("Completion error: {}", e))?;

  println!("\nGraph execution completed!");
  println!("The subgraph multiplied each value by 10 before dropping.");
  Ok(())
}
