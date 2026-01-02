//! Dead letter queue example
//!
//! This example demonstrates error handling using the ErrorBranch router
//! with a DeadLetterQueue consumer to collect failed items for later analysis.

use streamweave_array::ArrayProducer;
use streamweave_graph::control_flow::ErrorBranch;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, OutputRouterNode, ProducerNode,
};
use streamweave_vec::{DeadLetterQueue, VecConsumer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Dead Letter Queue Example");
  println!("========================\n");

  // Create a producer that generates items, some of which will fail processing
  let producer = ArrayProducer::new([
    Ok(1),
    Ok(2),
    Err("invalid input: negative number".to_string()),
    Ok(4),
    Err("invalid input: too large".to_string()),
    Ok(6),
  ]);

  // Create an ErrorBranch router to split success and error items
  let error_router = ErrorBranch::<i32, String>::new();

  // Create a DeadLetterQueue to collect failed items
  let dlq =
    DeadLetterQueue::<Result<i32, String>>::new().with_name("dead_letter_queue".to_string());

  // Create a success consumer to collect successful items
  let success_consumer =
    VecConsumer::<Result<i32, String>>::new().with_name("success_sink".to_string());

  // Build the graph
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer("source".to_string(), producer))?
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
      success_consumer,
    ))?
    .node(ConsumerNode::from_consumer(
      "dead_letter_queue".to_string(),
      dlq,
    ))?
    // Connect source to error splitter
    .connect_by_name("source", "error_split")?
    // Connect success port (port 0) to success sink
    .connect_by_name("error_split", "success_sink")?
    // Connect error port (port 1) to dead letter queue
    // ErrorBranch routes Ok items to port 0 and Err items to port 1
    .connect_by_name("error_split", "dead_letter_queue")?
    .build();

  // Execute the graph
  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("\nGraph execution completed!");
  println!("The ErrorBranch router routes:");
  println!("  - Ok(items) to 'success_sink' (port 0 = 'out')");
  println!("  - Err(errors) to 'dead_letter_queue' (port 1 = 'out_a')");
  println!("\nThe dead letter queue can be used to:");
  println!("  - Analyze failed items");
  println!("  - Retry processing");
  println!("  - Generate error reports");
  println!("  - Monitor error rates and patterns");

  Ok(())
}
