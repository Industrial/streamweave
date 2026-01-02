//! Execution lifecycle example
//!
//! This example demonstrates the graph execution lifecycle:
//! start, pause, resume, and stop operations.

use std::time::Duration;
use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
    ))?
    .node(TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "transform")?
    .connect_by_name("transform", "sink")?
    .build();

  let mut executor = graph.executor();

  // Start execution
  println!("Starting graph execution...");
  executor.start().await?;
  println!("Graph state: {:?}", executor.state());

  // Wait a bit
  tokio::time::sleep(Duration::from_millis(50)).await;

  // Pause execution
  println!("Pausing graph execution...");
  executor.pause().await?;
  println!("Graph state: {:?}", executor.state());

  // Wait while paused
  tokio::time::sleep(Duration::from_millis(50)).await;

  // Resume execution
  println!("Resuming graph execution...");
  executor.resume().await?;
  println!("Graph state: {:?}", executor.state());

  // Wait a bit more
  tokio::time::sleep(Duration::from_millis(50)).await;

  // Stop execution
  println!("Stopping graph execution...");
  executor.stop().await?;
  println!("Graph state: {:?}", executor.state());

  println!("Execution lifecycle example completed!");
  Ok(())
}
