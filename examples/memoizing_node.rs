//! # Memoizing map node (cache hit / miss)
//!
//! Demonstrates [`MemoizingMapNode`] with value-based caching ([`HashKeyExtractor`]):
//! repeated inputs with the same value are served from cache and skip recomputation.
//!
//! See [docs/incremental-recomputation.md](docs/incremental-recomputation.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node};
use streamweave::nodes::map_node::map_config;
use streamweave::nodes::memoizing_map_node::{HashKeyExtractor, MemoizingMapNode};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

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

  let node = MemoizingMapNode::with_key_extractor("memo".to_string(), Arc::new(HashKeyExtractor));
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs: InputStreams = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as streamweave::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as streamweave::node::InputStream,
  );

  let mut outputs = node.execute(inputs).await?;
  let mut out_stream = outputs.remove("out").unwrap();

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

  let mut results = Vec::new();
  while let Some(arc) = out_stream.next().await {
    if let Ok(n) = arc.downcast::<i32>() {
      results.push(*n);
    }
  }

  println!("Input:  [1, 2, 1, 2] (repeated values)");
  println!(
    "Output: {:?} (double; second 1 and 2 served from cache)",
    results
  );
  assert_eq!(results, vec![2, 4, 2, 4], "memoized double");
  println!("\nDone.");
  Ok(())
}
