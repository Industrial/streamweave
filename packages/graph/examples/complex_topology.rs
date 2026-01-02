//! Complex topology example
//!
//! This example demonstrates a complex graph topology with
//! multiple producers, transformers, and consumers with fan-in/fan-out.

use streamweave_array::ArrayProducer;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create a complex graph with fan-in and fan-out
  // Producer1, Producer2 -> [Merge] -> [Transform] -> [Broadcast] -> Consumer1, Consumer2, Consumer3
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
      "merger".to_string(),
      MapTransformer::new(|x: i32| x),
    ))?
    .node(TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))?
    .node(TransformerNode::from_transformer(
      "broadcast".to_string(),
      MapTransformer::new(|x: i32| x),
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
    .connect_by_name("source1", "merger")?
    .connect_by_name("source2", "merger")?
    .connect_by_name("merger", "transform")?
    .connect_by_name("transform", "broadcast")?
    .connect_by_name("broadcast", "consumer1")?
    .connect_by_name("broadcast", "consumer2")?
    .connect_by_name("broadcast", "consumer3")?
    .build();

  // Execute the graph
  let mut executor = graph.executor();

  executor.start().await?;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  executor.stop().await?;

  println!("Complex topology execution completed!");
  Ok(())
}
