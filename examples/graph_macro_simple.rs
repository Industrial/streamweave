//! # Simple Linear Pipeline Example
//!
//! This example demonstrates the `graph!` macro with a simple linear pipeline:
//! producer -> transform -> sink
//!
//! The `graph!` macro provides a concise, declarative syntax for building graphs,
//! reducing boilerplate compared to the traditional Graph API.

use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use async_trait::async_trait;
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};

// Simple producer node that emits numbers
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
    fn execute(&self, _inputs: InputStreams) -> Pin<Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
        let data = self.data.clone();
        Box::pin(async move {
            let (tx, rx) = mpsc::channel(10);
            tokio::spawn(async move {
                for item in data {
                    let _ = tx.send(Arc::new(item) as Arc<dyn Any + Send + Sync>).await;
                }
            });
            let mut outputs = HashMap::new();
            outputs.insert("out".to_string(), Box::pin(ReceiverStream::new(rx)) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>);
            Ok(outputs)
        })
    }
}

// Transform node that doubles the input values
struct TransformNode {
    name: String,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
}

impl TransformNode {
    fn new(name: String) -> Self {
        Self {
            name,
            input_port_names: vec!["in".to_string()],
            output_port_names: vec!["out".to_string()],
        }
    }
}

#[async_trait]
impl Node for TransformNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    fn input_port_names(&self) -> &[String] {
        &self.input_port_names
    }
    fn output_port_names(&self) -> &[String] {
        &self.output_port_names
    }
    fn has_input_port(&self, name: &str) -> bool {
        name == "in"
    }
    fn has_output_port(&self, name: &str) -> bool {
        name == "out"
    }
    fn execute(&self, mut inputs: InputStreams) -> Pin<Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
        Box::pin(async move {
            let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
            let mut outputs = HashMap::new();
            outputs.insert("out".to_string(), Box::pin(async_stream::stream! {
                let mut input = input_stream;
                while let Some(item) = input.next().await {
                    if let Ok(arc_i32) = item.clone().downcast::<i32>() {
                        yield Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>;
                    } else {
                        yield item;
                    }
                }
            }) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>);
            Ok(outputs)
        })
    }
}

// Sink node that collects results
struct SinkNode {
    name: String,
    input_port_names: Vec<String>,
    received: Arc<tokio::sync::Mutex<Vec<i32>>>,
}

impl SinkNode {
    fn new(name: String) -> Self {
        Self {
            name,
            input_port_names: vec!["in".to_string()],
            received: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl Node for SinkNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    fn input_port_names(&self) -> &[String] {
        &self.input_port_names
    }
    fn output_port_names(&self) -> &[String] {
        &[]
    }
    fn has_input_port(&self, name: &str) -> bool {
        name == "in"
    }
    fn has_output_port(&self, _name: &str) -> bool {
        false
    }
    fn execute(&self, mut inputs: InputStreams) -> Pin<Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
        let received = Arc::clone(&self.received);
        Box::pin(async move {
            let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
            tokio::spawn(async move {
                let mut input = input_stream;
                while let Some(item) = input.next().await {
                    if let Ok(arc_i32) = item.downcast::<i32>() {
                        received.lock().await.push(*arc_i32);
                    }
                }
            });
            Ok(HashMap::new())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Building graph with graph! macro...");
    
    // Use the graph! macro to create a linear pipeline
    // Syntax: node_name: NodeInstance
    // Connections: source_node.source_port => target_node.target_port
    let mut graph: Graph = graph! {
        producer: ProducerNode::new("producer".to_string(), vec![1, 2, 3, 4, 5]),
        transform: TransformNode::new("transform".to_string()),
        sink: SinkNode::new("sink".to_string()),
        producer.out => transform.in,
        transform.out => sink.in
    };
    
    
    println!("✓ Graph built with 3 nodes and 2 edges");
    println!("  - producer: emits numbers 1-5");
    println!("  - transform: doubles each number");
    println!("  - sink: collects results");
    
    // Execute the graph (use Graph::execute to disambiguate from Node::execute)
    println!("\nExecuting graph...");
    Graph::execute(&mut graph).await.map_err(|e| format!("Graph execution failed: {}", e))?;
    graph.wait_for_completion().await.map_err(|e| format!("Graph completion failed: {}", e))?;
    
    println!("✓ Graph execution completed");
    println!("\nThe graph processed numbers 1-5 through the pipeline:");
    println!("  - Producer emitted: [1, 2, 3, 4, 5]");
    println!("  - Transform doubled each: [2, 4, 6, 8, 10]");
    println!("  - Sink collected the results");
    println!("\n✓ Example completed successfully!");
    
    Ok(())
}
