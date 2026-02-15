//! # Memoizing map node (cache hit / miss)
//!
//! Demonstrates [`MemoizingMapNode`] with value-based caching ([`HashKeyExtractor`]):
//! repeated inputs with the same value are served from cache and skip recomputation.
//!
//! Graph is loaded from [memoizing_node.mmd](memoizing_node.mmd) (Style B Mermaid diagram).
//!
//! See [docs/incremental-recomputation.md](docs/incremental-recomputation.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use std::any::Any;
use std::path::Path;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::mermaid::{
  NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
};
use streamweave::nodes::map_node::map_config;
use streamweave::nodes::memoizing_map_node::{HashKeyExtractor, MemoizingMapNode};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!("Memoizing map node example (cache hit/miss with HashKeyExtractor)\n");

  let config = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  let path = Path::new("examples/memoizing_node.mmd");
  let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
  let mut registry = NodeRegistry::new();
  registry.register("MemoizingMapNode", |id, _inputs, _outputs| {
    Box::new(MemoizingMapNode::with_key_extractor(id, Arc::new(HashKeyExtractor)))
  });
  let mut graph = blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?;

  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel(10);

  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", data_rx)?;
  graph.connect_output_channel("output", out_tx)?;

  config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await?;
  drop(config_tx);
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  for x in [1i32, 2, 1, 2] {
    data_tx
      .send(Arc::new(x) as Arc<dyn Any + Send + Sync>)
      .await?;
  }
  drop(data_tx);

  graph.execute().await.map_err(|e| format!("{:?}", e))?;

  let mut results = Vec::new();
  while let Some(arc) = out_rx.recv().await {
    if let Ok(n) = arc.downcast::<i32>() {
      results.push(*n);
    }
  }
  graph.wait_for_completion().await?;

  println!("Input:  [1, 2, 1, 2] (repeated values)");
  println!(
    "Output: {:?} (double; second 1 and 2 served from cache)",
    results
  );
  assert_eq!(results, vec![2, 4, 2, 4], "memoized double");
  println!("\nDone.");
  Ok(())
}
