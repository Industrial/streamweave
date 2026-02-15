//! Tests for BoundedIterationNode

use crate::edge::Edge;
use crate::graph::Graph;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::bounded_iteration_node::BoundedIterationNode;
use crate::nodes::common::BaseNode;
use crate::nodes::stream::MergeNode;
use async_trait::async_trait;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

/// Test node: x -> x/2 + 1 (integer division). Used for fixed-point test.
struct HalfPlusOneNode(BaseNode);

impl HalfPlusOneNode {
    fn new() -> Self {
        Self(BaseNode::new(
            "half_plus_one".to_string(),
            vec!["in".to_string()],
            vec!["out".to_string()],
        ))
    }
}

#[async_trait]
impl Node for HalfPlusOneNode {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn set_name(&mut self, name: &str) {
        self.0.set_name(name);
    }
    fn input_port_names(&self) -> &[String] {
        self.0.input_port_names()
    }
    fn output_port_names(&self) -> &[String] {
        self.0.output_port_names()
    }
    fn has_input_port(&self, name: &str) -> bool {
        self.0.has_input_port(name)
    }
    fn has_output_port(&self, name: &str) -> bool {
        self.0.has_output_port(name)
    }
    fn execute(
        &self,
        mut inputs: InputStreams,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
        Box::pin(async move {
            let mut in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
            let (tx, rx) = mpsc::channel(64);
            tokio::spawn(async move {
                while let Some(item) = in_stream.next().await {
                    if let Ok(n) = item.downcast::<i32>() {
                        let out = *n / 2 + 1;
                        let _ = tx.send(Arc::new(out) as Arc<dyn std::any::Any + Send + Sync>).await;
                    }
                }
            });
            let mut out = HashMap::new();
            out.insert(
                "out".to_string(),
                Box::pin(ReceiverStream::new(rx)) as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>>,
            );
            Ok(out)
        })
    }
}

fn build_inner_graph() -> Graph {
    let mut g = Graph::new("inner".to_string());
    let merge = MergeNode::new_deterministic("merge".to_string(), 2);
    g.add_node("merge".to_string(), Box::new(merge)).unwrap();
    g.expose_input_port("merge", "in_0", "seed").unwrap();
    g.expose_input_port("merge", "in_1", "feedback").unwrap();
    g.expose_output_port("merge", "out", "output").unwrap();
    g
}

#[tokio::test]
async fn test_bounded_iteration_node_creation() {
    let inner = build_inner_graph();
    let node = BoundedIterationNode::new("iter".to_string(), inner, 3);
    assert_eq!(node.name(), "iter");
    assert!(node.has_input_port("seed"));
    assert!(node.has_output_port("output"));
    assert!(node.has_output_port("converged"));
}

#[tokio::test]
async fn test_bounded_iteration_single_round() {
    let inner = build_inner_graph();
    let node = BoundedIterationNode::new("iter".to_string(), inner, 1);

    let (seed_tx, seed_rx) = mpsc::channel(10);
    let mut inputs = HashMap::new();
    inputs.insert(
        "seed".to_string(),
        Box::pin(ReceiverStream::new(seed_rx)) as crate::node::InputStream,
    );

    let mut outputs = node.execute(inputs).await.unwrap();
    let mut out_stream = outputs.remove("output").unwrap();

    seed_tx.send(Arc::new(42i32) as Arc<dyn std::any::Any + Send + Sync>).await.unwrap();
    drop(seed_tx);

    let mut results = Vec::new();
    let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            _ = &mut timeout => break,
            Some(item) = out_stream.next() => {
                if let Ok(n) = item.clone().downcast::<i32>() {
                    results.push(*n);
                }
            }
        }
    }
    assert_eq!(results, vec![42]);
}

#[tokio::test]
async fn test_bounded_iteration_two_rounds() {
    let inner = build_inner_graph();
    let node = BoundedIterationNode::new("iter".to_string(), inner, 2);

    let (seed_tx, seed_rx) = mpsc::channel(10);
    let mut inputs = HashMap::new();
    inputs.insert(
        "seed".to_string(),
        Box::pin(ReceiverStream::new(seed_rx)) as crate::node::InputStream,
    );

    let mut outputs = node.execute(inputs).await.unwrap();
    let mut out_stream = outputs.remove("output").unwrap();

    seed_tx.send(Arc::new(7i32) as Arc<dyn std::any::Any + Send + Sync>).await.unwrap();
    drop(seed_tx);

    let mut results = Vec::new();
    let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            _ = &mut timeout => break,
            Some(item) = out_stream.next() => {
                if let Ok(n) = item.clone().downcast::<i32>() {
                    results.push(*n);
                }
            }
        }
    }
    assert_eq!(results, vec![7, 7]);
}

#[tokio::test]
async fn test_bounded_iteration_fixed_point() {
    let mut g = Graph::new("inner".to_string());
    let merge = MergeNode::new_deterministic("merge".to_string(), 2);
    let map = HalfPlusOneNode::new();
    g.add_node("merge".to_string(), Box::new(merge)).unwrap();
    g.add_node("map".to_string(), Box::new(map)).unwrap();
    g.add_edge(Edge {
        source_node: "merge".to_string(),
        source_port: "out".to_string(),
        target_node: "map".to_string(),
        target_port: "in".to_string(),
    }).unwrap();
    g.expose_input_port("merge", "in_0", "seed").unwrap();
    g.expose_input_port("merge", "in_1", "feedback").unwrap();
    g.expose_output_port("map", "out", "output").unwrap();

    let node = BoundedIterationNode::new("iter".to_string(), g, 5);

    let (seed_tx, seed_rx) = mpsc::channel(10);
    let mut inputs = HashMap::new();
    inputs.insert(
        "seed".to_string(),
        Box::pin(ReceiverStream::new(seed_rx)) as crate::node::InputStream,
    );

    let mut outputs = node.execute(inputs).await.unwrap();
    let mut out_stream = outputs.remove("output").unwrap();

    seed_tx.send(Arc::new(10i32) as Arc<dyn std::any::Any + Send + Sync>).await.unwrap();
    drop(seed_tx);

    let mut results = Vec::new();
    let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            _ = &mut timeout => break,
            Some(item) = out_stream.next() => {
                if let Ok(n) = item.clone().downcast::<i32>() {
                    results.push(*n);
                }
            }
        }
    }
    assert_eq!(results, vec![6, 4, 3, 2, 2]);
}
