//! While loop transformer example
//!
//! This example demonstrates the While transformer, which implements
//! conditional iteration (while loop pattern).

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode, control_flow::While,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with While transformer
  // Emits items when condition becomes false (x >= 5)
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 5, 4, 6, 7]),
    ))?
    .node(TransformerNode::from_transformer(
      "while_loop".to_string(),
      While::new(|x: &i32| *x < 5),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "while_loop")?
    .connect_by_name("while_loop", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("While loop transformer example completed!");
  println!("Items where condition is false (x >= 5) were emitted");
  Ok(())
}
