//! GroupBy transformer example
//!
//! This example demonstrates the GroupBy transformer, which groups
//! items by a key function.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode, control_flow::GroupBy,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with GroupBy transformer
  // Groups numbers by their value modulo 3
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9]),
    ))?
    .node(TransformerNode::from_transformer(
      "group_by".to_string(),
      GroupBy::new(|x: &i32| *x % 3),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<(i32, Vec<i32>)>::new(),
    ))?
    .connect_by_name("source", "group_by")?
    .connect_by_name("group_by", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("GroupBy transformer example completed!");
  println!("Items were grouped by key (value % 3)");
  Ok(())
}
