//! Match router example
//!
//! This example demonstrates pattern-based routing using the Match router
//! within a graph. The Match router routes items to different ports based
//! on pattern matching (ranges in this case).

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_graph::control_flow::{Match, Pattern, RangePattern};
use streamweave_graph::router::OutputRouter;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, OutputRouterNode, ProducerNode,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the Match router directly with streams
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
    Box::new(RangePattern::new(0..20, 0)),  // 0-19 -> port 0
    Box::new(RangePattern::new(20..40, 1)), // 20-39 -> port 1
  ];
  let mut match_router = Match::new(patterns, Some(2)); // default -> port 2

  let mut producer = ArrayProducer::new([1, 5, 15, 25, 35, 45, 55]);
  let input_stream = Box::pin(producer.produce());
  let mut output_streams = match_router.route_stream(input_stream).await;

  // Collect results from all ports
  let mut port0_results = Vec::new();
  let mut port1_results = Vec::new();
  let mut port2_results = Vec::new();

  for (port_name, stream) in &mut output_streams {
    let s = stream;
    while let Some(item) = s.next().await {
      match port_name.as_str() {
        "out_0" => port0_results.push(item),
        "out_1" => port1_results.push(item),
        "out_2" => port2_results.push(item),
        _ => {}
      }
    }
  }

  println!("Match router demonstration:");
  println!("  Low range (0-19, port 0): {:?}", port0_results);
  println!("  Medium range (20-39, port 1): {:?}", port1_results);
  println!("  High range (40+, port 2): {:?}", port2_results);
  assert_eq!(port0_results, vec![1, 5, 15]);
  assert_eq!(port1_results, vec![25, 35]);
  assert_eq!(port2_results, vec![45, 55]);

  // Now demonstrate using a graph with the Match router as a node
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
    Box::new(RangePattern::new(0..20, 0)),  // 0-19 -> port 0
    Box::new(RangePattern::new(20..40, 1)), // 20-39 -> port 1
  ];
  let match_router = Match::new(patterns, Some(2)); // default -> port 2
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 5, 15, 25, 35, 45, 55]),
    ))?
    .node(
      OutputRouterNode::<Match<i32>, i32, (i32,), (i32, i32, i32)>::from_router(
        "match".to_string(),
        match_router,
        vec!["in".to_string()],
        vec![
          "out_0".to_string(),
          "out_1".to_string(),
          "out_2".to_string(),
        ],
      ),
    )?
    .node(ConsumerNode::from_consumer(
      "low_sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "medium_sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "high_sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "match")?
    .connect_by_name("match", "low_sink")?
    .connect_by_name("match", "medium_sink")?
    .connect_by_name("match", "high_sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("The Match router node routes items based on ranges to the appropriate sinks");
  Ok(())
}
