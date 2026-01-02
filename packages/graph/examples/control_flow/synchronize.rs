//! Synchronize input router example
//!
//! This example demonstrates the Synchronize input router within a graph,
//! which synchronizes multiple input streams, waiting for all before proceeding.

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_graph::control_flow::Synchronize;
use streamweave_graph::router::InputRouter;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, InputRouterNode, ProducerNode,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the Synchronize router directly with streams
  let mut sync_router = Synchronize::new(2);

  let mut producer1 = ArrayProducer::new([1, 2, 3]);
  let mut producer2 = ArrayProducer::new([4, 5, 6]);
  let stream1 = producer1.produce();
  let stream2 = producer2.produce();

  let expected_ports = sync_router.expected_port_names();
  let streams = vec![
    (expected_ports[0].clone(), stream1),
    (expected_ports[1].clone(), stream2),
  ];
  let mut output_stream = sync_router.route_streams(streams).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  println!("Synchronize router demonstration:");
  println!("  Synchronized results: {:?}", results);
  println!("  Items from both streams were synchronized");

  // Now demonstrate using a graph with the Synchronize router as a node
  let sync_router = Synchronize::new(2);
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source1".to_string(),
      ArrayProducer::new([1, 2, 3]),
    ))?
    .node(ProducerNode::from_producer(
      "source2".to_string(),
      ArrayProducer::new([4, 5, 6]),
    ))?
    .node(
      InputRouterNode::<Synchronize<i32>, i32, (i32, i32), (i32,)>::from_router(
        "sync".to_string(),
        sync_router,
        vec!["in".to_string(), "in_1".to_string()],
        vec!["out".to_string()],
      ),
    )?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source1", "sync")?
    .connect_by_name("source2", "sync")?
    .connect_by_name("sync", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("The Synchronize router node synchronizes multiple input streams");
  println!("before processing");
  Ok(())
}
