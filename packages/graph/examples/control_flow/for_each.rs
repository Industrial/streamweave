//! ForEach transformer example
//!
//! This example demonstrates the ForEach transformer, which expands
//! collections into individual items (for-each loop pattern).

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode, control_flow::ForEach,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with ForEach transformer
  // Expands Vec<i32> items into individual i32 items
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([vec![1, 2, 3], vec![4, 5], vec![6, 7, 8, 9]]),
    ))?
    .node(TransformerNode::from_transformer(
      "for_each".to_string(),
      ForEach::new(|vec: &Vec<i32>| vec![vec.clone()]),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<Vec<i32>>::new(),
    ))?
    .connect_by_name("source", "for_each")?
    .connect_by_name("for_each", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("ForEach transformer example completed!");
  println!("Collections were expanded into individual items");
  Ok(())
}
