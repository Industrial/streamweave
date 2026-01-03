//! Integration tests for README examples
//!
//! This module verifies that all code examples in the README compile and run correctly.

use streamweave::graph::{
  GraphBuilder, GraphExecution,
  nodes::{ConsumerNode, ProducerNode, TransformerNode},
};
use streamweave_array::ArrayProducer;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::test]
async fn test_readme_simple_graph() {
  // Test the Simple Graph example from README
  let mut builder = GraphBuilder::new();

  // Add nodes
  builder
    .node(ProducerNode::from_producer(
      "source".to_string(),
      ArrayProducer::new(vec![1, 2, 3]),
    ))
    .unwrap();

  builder
    .node(TransformerNode::from_transformer(
      "mapper".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))
    .await
    .unwrap();

  builder
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::new(),
    ))
    .unwrap();

  // Connect nodes
  builder.connect_by_name("source", "mapper").unwrap();
  builder.connect_by_name("mapper", "sink").unwrap();

  // Build and execute
  let graph = builder.build();
  let mut executor = graph.executor();
  executor.start().await.unwrap();

  // Wait a bit for processing
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  executor.stop().await.unwrap();

  // Verify no errors occurred
  assert!(executor.errors().is_empty());
}

#[tokio::test]
async fn test_readme_graph_builder_basic() {
  // Test basic GraphBuilder usage
  let builder = GraphBuilder::new();
  let graph = builder.build();

  // Verify we can create an executor
  let _executor = graph.executor();
}
