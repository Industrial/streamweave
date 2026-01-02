//! Compression example
//!
//! This example demonstrates how to use compression
//! for data transmission between nodes in distributed execution mode.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  execution::{CompressionAlgorithm, ExecutionMode},
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

  // Configure compression in distributed execution mode
  // Use gzip compression with level 6
  let compression = CompressionAlgorithm::Gzip { level: 6 };
  let execution_mode = ExecutionMode::Distributed {
    serializer: streamweave_graph::serialization::JsonSerializer,
    compression: Some(compression),
    batching: None,
  };

  // Execute the graph with compression enabled
  let mut executor = graph.with_execution_mode(execution_mode).executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Compression example completed!");
  println!("Data was compressed using gzip (level 6) during transmission");
  Ok(())
}
