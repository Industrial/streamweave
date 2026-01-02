//! Subgraph example
//!
//! This example demonstrates how to create and use subgraphs
//! (nested graphs) for modular graph construction.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, SubgraphNode, TransformerNode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create an inner graph (subgraph)
  let inner_graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "inner_source".to_string(),
      ArrayProducer::new([1, 2, 3]),
    ))?
    .node(TransformerNode::from_transformer(
      "inner_transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))?
    .connect_by_name("inner_source", "inner_transform")?
    .build();

  // Create a subgraph node with 0 inputs and 1 output
  let mut subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    inner_graph,
    vec!["in".to_string()],
    vec!["out".to_string()],
  );

  // Map the subgraph's output port "out" to the inner graph's "inner_transform" output
  subgraph.map_output_port("out", "inner_transform".to_string(), "out")?;

  // Create the outer graph with the subgraph
  let outer_graph = GraphBuilder::new()
    .add_node("subgraph".to_string(), subgraph)?
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))?
    .connect_by_name("subgraph", "sink")?
    .build();

  // Execute the graph
  let mut executor = outer_graph.executor();
  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
  executor.stop().await?;

  println!("Subgraph execution completed!");
  Ok(())
}
