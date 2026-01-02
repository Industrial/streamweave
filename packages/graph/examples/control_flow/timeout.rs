//! Timeout transformer example
//!
//! This example demonstrates the Timeout transformer within a graph, which wraps
//! items in Result<T, TimeoutError> based on processing time.

use futures::StreamExt;
use std::time::Duration;
use streamweave::Producer;
use streamweave::Transformer;
use streamweave_array::ArrayProducer;
use streamweave_graph::control_flow::Timeout;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the Timeout transformer directly with streams
  let mut timeout = Timeout::<i32>::new(Duration::from_millis(50));

  let mut producer = ArrayProducer::new([1, 2, 3, 4, 5]);
  let input_stream = Box::pin(producer.produce());
  let output_stream = timeout.transform(input_stream).await;

  let mut results = Vec::new();
  let mut stream = output_stream;
  while let Some(result) = stream.next().await {
    results.push(result);
  }

  println!("Timeout transformer demonstration:");
  println!(
    "  Results: {} items wrapped in Result<i32, TimeoutError>",
    results.len()
  );
  println!("  Items were wrapped based on processing time");

  // Now demonstrate using a graph (note: TimeoutError needs Serialize/Deserialize
  // for full graph integration, so this shows the graph structure)
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5]),
    ))?
    .node(TransformerNode::from_transformer(
      "transform".to_string(),
      streamweave_transformers::MapTransformer::new(|x: i32| x * 2),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "transform")?
    .connect_by_name("transform", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(Duration::from_millis(200)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("Note: For full graph integration, TimeoutError needs to implement");
  println!("Serialize/Deserialize. The Timeout transformer can be used in");
  println!("in-process execution mode where serialization isn't required.");
  Ok(())
}
