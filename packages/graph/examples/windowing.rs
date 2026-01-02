//! Windowing example
//!
//! This example demonstrates how to use windowing operations
//! in a graph (tumbling and sliding windows).

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_vec::VecConsumer;
use streamweave_window::WindowTransformer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with a tumbling window (count-based)
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
    ))?
    .node(TransformerNode::from_transformer(
      "window".to_string(),
      WindowTransformer::<i32>::new(3), // Window size of 3
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<Vec<i32>>::new(),
    ))?
    .connect_by_name("source", "window")?
    .connect_by_name("window", "sink")?
    .build();

  // Execute the graph
  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Windowing completed! Items grouped into windows of size 3.");
  Ok(())
}
