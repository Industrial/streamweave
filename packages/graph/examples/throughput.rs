//! Throughput monitoring example
//!
//! This example demonstrates how to monitor throughput
//! (items per second) in a graph using ThroughputMonitor.

use std::time::Duration;
use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  throughput::ThroughputMonitor,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a throughput monitor
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

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

  // Execute the graph
  let mut executor = graph.executor();

  // Simulate item processing and increment throughput monitor
  // In a real scenario, nodes would call monitor.increment_item_count()
  for _ in 0..10 {
    monitor.increment_item_count();
  }

  executor.start().await?;
  tokio::time::sleep(Duration::from_millis(200)).await;
  executor.stop().await?;

  // Calculate and display throughput
  let throughput = monitor.calculate_throughput().await;
  let item_count = monitor.item_count();

  println!("Throughput monitoring example completed!");
  println!("Total items processed: {}", item_count);
  println!("Throughput: {:.2} items/second", throughput);

  Ok(())
}
