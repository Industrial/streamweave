//! If router example
//!
//! This example demonstrates conditional routing using the If router
//! within a graph. The If router routes items to different ports based
//! on a predicate (even numbers to one consumer, odd to another).

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_graph::router::OutputRouter;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, OutputRouterNode, ProducerNode, control_flow::If,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the If router directly with streams
  let mut if_router = If::new(|x: &i32| *x % 2 == 0);
  let mut producer = ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  let input_stream = Box::pin(producer.produce());
  let mut output_streams = if_router.route_stream(input_stream).await;

  // Collect results from both ports
  let mut even_results = Vec::new();
  let mut odd_results = Vec::new();

  for (port_name, stream) in &mut output_streams {
    let s = stream;
    if port_name == "true" {
      while let Some(item) = s.next().await {
        even_results.push(item);
      }
    } else if port_name == "false" {
      while let Some(item) = s.next().await {
        odd_results.push(item);
      }
    }
  }

  println!("If router demonstration:");
  println!("  Even numbers (port 0): {:?}", even_results);
  println!("  Odd numbers (port 1): {:?}", odd_results);
  assert_eq!(even_results, vec![2, 4, 6, 8, 10]);
  assert_eq!(odd_results, vec![1, 3, 5, 7, 9]);

  // Now demonstrate using a graph with the If router as a node
  let if_router = If::new(|x: &i32| *x % 2 == 0);
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
    ))?
    .node(
      OutputRouterNode::<If<i32>, i32, (i32,), (i32, i32)>::from_router(
        "split".to_string(),
        if_router,
        vec!["in".to_string()],
        vec!["true".to_string(), "false".to_string()],
      ),
    )?
    .node(ConsumerNode::from_consumer(
      "even_sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "odd_sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "split")?
    .connect_by_name("split", "even_sink")?
    .connect_by_name("split", "odd_sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("The If router node routes even numbers to 'even_sink' and odd to 'odd_sink'");
  Ok(())
}
