//! # Memoizing map node (cache hit / miss)
//!
//! Demonstrates [`MemoizingMapNode`] with value-based caching ([`HashKeyExtractor`]):
//! repeated inputs with the same value are served from cache and skip recomputation.
//!
//! See [docs/incremental-recomputation.md](docs/incremental-recomputation.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::map_node::map_config;
use streamweave::nodes::memoizing_map_node::{HashKeyExtractor, MemoizingMapNode};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};

/// Producer that emits a fixed sequence of i32 values.
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
                // Small delay so memo node receives config before first data item
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                for x in data {
                    let _ = tx
                        .send(Arc::new(x) as Arc<dyn Any + Send + Sync>)
                        .await;
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
    println!("Memoizing map node example (cache hit/miss with HashKeyExtractor)\n");

    let double_config = map_config(|value| async move {
        if let Ok(arc_i32) = value.downcast::<i32>() {
            Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
        } else {
            Err("Expected i32".to_string())
        }
    });

    let mut graph = Graph::new("memo_demo".to_string());
    graph
        .add_node(
            "producer".to_string(),
            Box::new(ProducerNode::new(
                "producer".to_string(),
                vec![1, 2, 1, 2], // repeated 1, 2 -> second pair is cache hit with HashKeyExtractor
            )),
        )
        .unwrap();
    graph
        .add_node(
            "memo".to_string(),
            Box::new(MemoizingMapNode::with_key_extractor(
                "memo".to_string(),
                Arc::new(HashKeyExtractor),
            )),
        )
        .unwrap();
    graph
        .add_edge(Edge {
            source_node: "producer".to_string(),
            source_port: "out".to_string(),
            target_node: "memo".to_string(),
            target_port: "in".to_string(),
        })
        .unwrap();
    graph
        .expose_input_port("memo", "configuration", "config")
        .unwrap();
    graph
        .expose_output_port("memo", "out", "output")
        .unwrap();

    let (config_tx, config_rx) = mpsc::channel(10);
    graph.connect_input_channel("config", config_rx).unwrap();
    let (out_tx, mut out_rx) = mpsc::channel(10);
    graph.connect_output_channel("output", out_tx).unwrap();

    config_tx
        .send(Arc::new(double_config) as Arc<dyn Any + Send + Sync>)
        .await?;
    drop(config_tx);

    streamweave::graph::Graph::execute(&mut graph).await?;

    let mut results = Vec::new();
    while let Some(arc) = out_rx.recv().await {
        if let Ok(n) = arc.downcast::<i32>() {
            results.push(*n);
        }
    }
    graph.wait_for_completion().await?;

    println!("Input:  [1, 2, 1, 2] (repeated values)");
    println!("Output: {:?} (double; second 1 and 2 served from cache)", results);
    assert_eq!(results, vec![2, 4, 2, 4], "memoized double");
    println!("\nDone.");
    Ok(())
}
