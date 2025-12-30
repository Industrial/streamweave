//! Node type tests
//!
//! This module provides integration tests for node types (ProducerNode,
//! TransformerNode, ConsumerNode) that complement the unit tests in node.rs.

use streamweave_graph::{ConsumerNode, ProducerNode, TransformerNode};
use streamweave_transformers::MapTransformer;
use streamweave_vec::{VecConsumer, VecProducer};

#[test]
fn test_producer_node_creation() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  assert_eq!(node.name(), "source");
  assert_eq!(node.node_kind(), streamweave_graph::NodeKind::Producer);
}

#[test]
fn test_producer_node_from_producer() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node = ProducerNode::from_producer("source".to_string(), producer);

  assert_eq!(node.name(), "source");
}

#[test]
fn test_transformer_node_creation() {
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  assert_eq!(node.name(), "mapper");
  assert_eq!(node.node_kind(), streamweave_graph::NodeKind::Transformer);
}

#[test]
fn test_transformer_node_from_transformer() {
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node = TransformerNode::from_transformer("mapper".to_string(), transformer);

  assert_eq!(node.name(), "mapper");
}

#[test]
fn test_consumer_node_creation() {
  let consumer = VecConsumer::<i32>::new();
  let node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer);

  assert_eq!(node.name(), "sink");
  assert_eq!(node.node_kind(), streamweave_graph::NodeKind::Consumer);
}

#[test]
fn test_consumer_node_from_consumer() {
  let consumer = VecConsumer::<i32>::new();
  let node = ConsumerNode::from_consumer("sink".to_string(), consumer);

  assert_eq!(node.name(), "sink");
}

#[test]
fn test_node_with_name() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> =
    ProducerNode::new("original".to_string(), producer).with_name("new_name".to_string());

  assert_eq!(node.name(), "new_name");
}

#[test]
fn test_node_accessors() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let mut node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // Test that we can access the producer
  let _producer_ref = node.producer();
  let _producer_mut = node.producer_mut();
}

#[test]
fn test_transformer_node_accessors() {
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let mut node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  // Test that we can access the transformer
  let _transformer_ref = node.transformer();
  let _transformer_mut = node.transformer_mut();
}

#[test]
fn test_consumer_node_accessors() {
  let consumer = VecConsumer::<i32>::new();
  let mut node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer);

  // Test that we can access the consumer
  let _consumer_ref = node.consumer();
  let _consumer_mut = node.consumer_mut();
}

#[test]
fn test_node_port_counts() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  assert_eq!(node.input_port_count(), 0);
  assert_eq!(node.output_port_count(), 1);

  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  assert_eq!(node.input_port_count(), 1);
  assert_eq!(node.output_port_count(), 1);

  let consumer = VecConsumer::<i32>::new();
  let node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer);

  assert_eq!(node.input_port_count(), 1);
  assert_eq!(node.output_port_count(), 0);
}

#[test]
fn test_node_port_resolution() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // Producer has no input ports
  assert_eq!(node.resolve_input_port("in0"), None);

  // Producer has one output port
  assert_eq!(node.resolve_output_port("out0"), Some(0));
  assert_eq!(node.resolve_output_port("out"), Some(0));
  assert_eq!(node.resolve_output_port("0"), Some(0));

  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  // Transformer has input and output ports
  assert_eq!(node.resolve_input_port("in0"), Some(0));
  assert_eq!(node.resolve_input_port("in"), Some(0));
  assert_eq!(node.resolve_output_port("out0"), Some(0));
  assert_eq!(node.resolve_output_port("out"), Some(0));
}

#[test]
fn test_node_port_names() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  assert_eq!(node.input_port_name(0), None); // No input ports
  assert_eq!(node.output_port_name(0), Some("out0".to_string()));
  assert_eq!(node.output_port_name(1), None);

  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  assert_eq!(node.input_port_name(0), Some("in0".to_string()));
  assert_eq!(node.input_port_name(1), None);
  assert_eq!(node.output_port_name(0), Some("out0".to_string()));
  assert_eq!(node.output_port_name(1), None);
}
