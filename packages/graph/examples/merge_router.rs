//! Merge router example
//!
//! This example demonstrates the MergeRouter within a graph, which combines
//! multiple input streams into one output (fan-in pattern).

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_graph::merge_router::{MergeRouter, MergeStrategy};
use streamweave_graph::router::InputRouter;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the MergeRouter directly with streams
  let mut merge_router = MergeRouter::new(vec![0, 1], MergeStrategy::Interleave);

  let mut producer1 = ArrayProducer::new([1, 2, 3]);
  let mut producer2 = ArrayProducer::new([4, 5, 6]);
  let stream1 = producer1.produce();
  let stream2 = producer2.produce();

  let expected_ports = merge_router.expected_port_names();
  let streams = vec![
    (expected_ports[0].clone(), stream1),
    (expected_ports[1].clone(), stream2),
  ];
  let mut output_stream = merge_router.route_streams(streams).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  println!("MergeRouter demonstration:");
  println!("  Merged results: {:?}", results);
  println!("  Multiple streams were merged using interleave strategy");
  assert_eq!(results.len(), 6);

  // Now demonstrate using a graph with fan-in pattern
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source1".to_string(),
      ArrayProducer::new([1, 2, 3]),
    ))?
    .node(ProducerNode::from_producer(
      "source2".to_string(),
      ArrayProducer::new([4, 5, 6]),
    ))?
    .node(TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source1", "transform")?
    .connect_by_name("source2", "transform")?
    .connect_by_name("transform", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("In a future version, the MergeRouter will be configured as");
  println!("the input router for the 'transform' transformer to merge");
  println!("multiple input streams before processing");
  Ok(())
}
