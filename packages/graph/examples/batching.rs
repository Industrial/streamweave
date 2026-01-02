//! Batching example
//!
//! This example demonstrates how to use batching in distributed execution mode
//! to group items together for batch processing.

use std::time::Duration;
use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  execution::{BatchConfig, ExecutionMode},
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

  // Configure batching in distributed execution mode
  // Batch size of 5 items, timeout of 100ms
  let batch_config = BatchConfig::new(5, 100);
  let execution_mode = ExecutionMode::Distributed {
    serializer: streamweave_graph::serialization::JsonSerializer,
    compression: None,
    batching: Some(batch_config),
  };

  // Execute the graph with batching enabled
  let mut executor = graph.with_execution_mode(execution_mode).executor();
  executor.start().await?;
  tokio::time::sleep(Duration::from_millis(200)).await;
  executor.stop().await?;

  println!("Batching example completed!");
  println!("Items were processed in batches of 5 with 100ms timeout");
  Ok(())
}
