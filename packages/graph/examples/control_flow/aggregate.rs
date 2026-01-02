//! Aggregate transformer example
//!
//! This example demonstrates the Aggregate transformer with different
//! aggregators (Sum, Count, Min, Max).

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  control_flow::{Aggregate, SumAggregator},
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a graph with SumAggregator
  // Aggregates items in windows of 3
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9]),
    ))?
    .node(TransformerNode::from_transformer(
      "sum".to_string(),
      Aggregate::<i32, i32>::new(SumAggregator, Some(3)),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "sum")?
    .connect_by_name("sum", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Aggregate transformer example completed!");
  println!("Items were aggregated using SumAggregator in windows of 3");
  println!("Other aggregators: CountAggregator, MinAggregator, MaxAggregator");
  Ok(())
}
