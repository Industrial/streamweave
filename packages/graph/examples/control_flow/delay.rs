//! Delay transformer example
//!
//! This example demonstrates the Delay transformer, which adds
//! a delay before emitting each item (useful for rate limiting).

use std::time::Duration;
use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode, control_flow::Delay,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with Delay transformer
  // Adds a 50ms delay before emitting each item
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5]),
    ))?
    .node(TransformerNode::from_transformer(
      "delay".to_string(),
      Delay::<i32>::new(Duration::from_millis(50)),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "delay")?
    .connect_by_name("delay", "sink")?
    .build();

  let start = std::time::Instant::now();
  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
  executor.stop().await?;
  let elapsed = start.elapsed();

  println!("Delay transformer example completed!");
  println!(
    "Items were delayed by 50ms each (total elapsed: {:?})",
    elapsed
  );
  Ok(())
}
