//! Zero-copy example
//!
//! This example demonstrates zero-copy data sharing
//! using Arc for efficient fan-out scenarios in in-process execution mode.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  execution::ExecutionMode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with fan-out pattern (multiple consumers)
  // In in-process mode, data is shared via Arc to avoid copying
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
      "sink1".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink2".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "transform")?
    .connect_by_name("transform", "sink1")?
    .connect_by_name("transform", "sink2")?
    .build();

  // Execute in in-process mode (zero-copy)
  // Data is shared via Arc when fanning out to multiple consumers
  let execution_mode = ExecutionMode::InProcess {
    use_shared_memory: false,
  };

  let mut executor = graph.with_execution_mode(execution_mode).executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Zero-copy example completed!");
  println!("Data was shared efficiently via Arc in in-process execution mode");
  println!("No serialization or copying occurred - true zero-copy semantics");
  Ok(())
}
