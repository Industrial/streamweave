//! Basic graph construction example
//!
//! This example demonstrates how to create a simple linear graph with
//! a producer, transformer, and consumer.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a simple linear graph: producer -> transformer -> consumer
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5]),
    ))?
    .node(TransformerNode::from_transformer(
      "double".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "double")?
    .connect_by_name("double", "sink")?
    .build();

  // Execute the graph
  let mut executor = graph.executor();
  executor.start().await?;

  // Wait for completion
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  executor.stop().await?;

  println!("Graph execution completed successfully!");
  Ok(())
}
