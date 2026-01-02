//! Error branch router example
//!
//! This example demonstrates error handling using the ErrorBranch router
//! within a graph. The ErrorBranch router routes Result items to success
//! or error ports.

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_graph::control_flow::ErrorBranch;
use streamweave_graph::router::OutputRouter;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, OutputRouterNode, ProducerNode,
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // First, demonstrate the ErrorBranch router directly with streams
  let mut error_router = ErrorBranch::<i32, String>::new();

  let mut producer = ArrayProducer::new([
    Ok(1),
    Err("error1".to_string()),
    Ok(2),
    Err("error2".to_string()),
    Ok(3),
  ]);
  let input_stream = Box::pin(producer.produce());
  let mut output_streams = error_router.route_stream(input_stream).await;

  // Collect results from both ports
  let mut success_results = Vec::new();
  let mut error_results = Vec::new();

  for (port_name, stream) in &mut output_streams {
    let s = stream;
    if port_name == "success" {
      while let Some(result) = s.next().await {
        success_results.push(result);
      }
    } else if port_name == "error" {
      while let Some(result) = s.next().await {
        error_results.push(result);
      }
    }
  }

  println!("ErrorBranch router demonstration:");
  println!(
    "  Success results (port 0): {} items",
    success_results.len()
  );
  println!("  Error results (port 1): {} items", error_results.len());
  assert_eq!(success_results.len(), 3);
  assert_eq!(error_results.len(), 2);
  assert!(success_results.iter().all(|r| r.is_ok()));
  assert!(error_results.iter().all(|r| r.is_err()));

  // Now demonstrate using a graph with the ErrorBranch router as a node
  let error_router = ErrorBranch::<i32, String>::new();
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([
        Ok(1),
        Err("error1".to_string()),
        Ok(2),
        Err("error2".to_string()),
        Ok(3),
      ]),
    ))?
    .node(OutputRouterNode::<
      ErrorBranch<i32, String>,
      Result<i32, String>,
      (Result<i32, String>,),
      (Result<i32, String>, Result<i32, String>),
    >::from_router(
      "error_split".to_string(),
      error_router,
      vec!["in".to_string()],
      vec!["success".to_string(), "error".to_string()],
    ))?
    .node(ConsumerNode::from_consumer(
      "success_sink".to_string(),
      VecConsumer::<Result<i32, String>>::new(),
    ))?
    .node(ConsumerNode::from_consumer(
      "error_sink".to_string(),
      VecConsumer::<Result<i32, String>>::new(),
    ))?
    .connect_by_name("source", "error_split")?
    .connect_by_name("error_split", "success_sink")?
    .connect_by_name("error_split", "error_sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("The ErrorBranch router node routes Ok(items) to 'success_sink'");
  println!("and Err(errors) to 'error_sink'");
  Ok(())
}
