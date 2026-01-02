//! Round-robin router example
//!
//! This example demonstrates the RoundRobinRouter within a graph, which
//! distributes items evenly across multiple output ports (load balancing).

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_graph::RoundRobinRouter;
use streamweave_graph::router::OutputRouter;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the RoundRobinRouter directly with streams
  let mut round_robin_router = RoundRobinRouter::new(vec![0, 1, 2]);

  let mut producer = ArrayProducer::new([1, 2, 3, 4, 5, 6]);
  let input_stream = Box::pin(producer.produce());
  let mut output_streams = round_robin_router.route_stream(input_stream).await;

  // Collect results from all ports
  let mut port0_results = Vec::new();
  let mut port1_results = Vec::new();
  let mut port2_results = Vec::new();

  for (port_name, stream) in &mut output_streams {
    let s = stream;
    while let Some(item) = s.next().await {
      match port_name.as_str() {
        "out" => port0_results.push(item),
        "out_1" => port1_results.push(item),
        "out_2" => port2_results.push(item),
        _ => {}
      }
    }
  }

  println!("RoundRobinRouter demonstration:");
  println!("  Port 0 results: {:?}", port0_results);
  println!("  Port 1 results: {:?}", port1_results);
  println!("  Port 2 results: {:?}", port2_results);
  assert_eq!(port0_results, vec![1, 4]);
  assert_eq!(port1_results, vec![2, 5]);
  assert_eq!(port2_results, vec![3, 6]);

  // Now demonstrate using a graph with load balancing
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5, 6]),
    ))?
    .node(TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))?
    .node(ConsumerNode::from_consumer(
      "consumer1".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "consumer2".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "consumer3".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "transform")?
    .connect_by_name("transform", "consumer1")?
    .connect_by_name("transform", "consumer2")?
    .connect_by_name("transform", "consumer3")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("In a future version, the RoundRobinRouter will be configured on");
  println!("the 'transform' transformer to distribute items evenly across consumers");
  Ok(())
}
