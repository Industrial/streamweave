//! Shared memory channel example
//!
//! This example demonstrates how to use shared memory channels
//! for high-performance inter-process communication.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  execution::ExecutionMode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5]),
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

  // Configure shared memory channels in in-process execution mode
  let execution_mode = ExecutionMode::InProcess {
    use_shared_memory: true,
  };

  // Execute the graph with shared memory enabled
  let mut executor = graph.with_execution_mode(execution_mode).executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Shared memory channel example completed!");
  println!("Data was transmitted using shared memory channels for ultra-high performance");
  Ok(())
}
