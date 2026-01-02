//! Serialization example
//!
//! This example demonstrates serialization of data flowing through
//! the graph. In distributed execution mode, data is serialized
//! for transmission between nodes.

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
      ArrayProducer::new([1, 2, 3]),
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

  // Execute in distributed mode to demonstrate serialization
  // Data is automatically serialized using JSON when passing between nodes
  let execution_mode = ExecutionMode::Distributed {
    serializer: streamweave_graph::serialization::JsonSerializer,
    compression: None,
    batching: None,
  };

  let mut executor = graph.with_execution_mode(execution_mode).executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Serialization example completed!");
  println!("Data was serialized using JSON in distributed execution mode");
  println!("This enables nodes to run in separate processes or machines");
  Ok(())
}
