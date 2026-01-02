//! Variables example
//!
//! This example demonstrates GraphVariables, ReadVariable, and WriteVariable
//! for shared state management across nodes.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
  control_flow::{GraphVariables, ReadVariable, WriteVariable},
};
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create shared variables
  let vars = GraphVariables::new();
  vars.set("counter", 0i32);

  // Create a graph with WriteVariable and ReadVariable
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new([1, 2, 3, 4, 5]),
    ))?
    .node(TransformerNode::from_transformer(
      "write_var".to_string(),
      WriteVariable::<i32>::new("counter".to_string(), vars.clone()),
    ))?
    .node(TransformerNode::from_transformer(
      "read_var".to_string(),
      ReadVariable::<i32>::new("counter".to_string(), vars.clone()),
    ))?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("source", "write_var")?
    .connect_by_name("write_var", "read_var")?
    .connect_by_name("read_var", "sink")?
    .build();

  let mut executor = graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  // Check final variable value
  let final_value = vars.get::<i32>("counter");
  println!("Variables example completed!");
  println!("Final counter value: {:?}", final_value);
  Ok(())
}
