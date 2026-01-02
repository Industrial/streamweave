//! Join transformer example
//!
//! This example demonstrates the Join transformer, which joins
//! two streams using different strategies (Inner, Outer, Left, Right).

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  control_flow::{Join, JoinStrategy},
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with Join transformer
  // Joins tuples using Inner strategy
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([(1, 10), (2, 20), (3, 30)]),
    ))?
    .node(TransformerNode::from_transformer(
      "join".to_string(),
      Join::<i32, i32>::new(JoinStrategy::Inner),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<(i32, i32)>::new(),
    ))?
    .connect_by_name("source", "join")?
    .connect_by_name("join", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Join transformer example completed!");
  println!("Join strategies: Inner, Outer, Left, Right");
  Ok(())
}
