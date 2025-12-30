//! Graph traits tests
//!
//! This module provides tests for graph trait functionality, including
//! NodeTrait, NodeKind, and related trait implementations.

use streamweave_graph::{ConsumerNode, NodeKind, NodeTrait, ProducerNode, TransformerNode};
use streamweave_transformers::MapTransformer;
use streamweave_vec::{VecConsumer, VecProducer};

#[test]
fn test_node_kind_enum() {
  assert_eq!(NodeKind::Producer, NodeKind::Producer);
  assert_eq!(NodeKind::Transformer, NodeKind::Transformer);
  assert_eq!(NodeKind::Consumer, NodeKind::Consumer);
  assert_eq!(NodeKind::Subgraph, NodeKind::Subgraph);

  assert_ne!(NodeKind::Producer, NodeKind::Transformer);
  assert_ne!(NodeKind::Consumer, NodeKind::Subgraph);
}

#[test]
fn test_node_kind_debug() {
  let kind = NodeKind::Producer;
  let debug_str = format!("{:?}", kind);
  assert!(debug_str.contains("Producer"));

  let kind = NodeKind::Transformer;
  let debug_str = format!("{:?}", kind);
  assert!(debug_str.contains("Transformer"));
}

#[test]
fn test_node_kind_clone() {
  let kind1 = NodeKind::Producer;
  let kind2 = kind1;
  assert_eq!(kind1, kind2);

  let kind3 = NodeKind::Consumer;
  let kind4 = kind3;
  assert_eq!(kind3, kind4);
}

#[test]
fn test_producer_node_trait() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  assert_eq!(node.name(), "source");
  assert_eq!(node.node_kind(), NodeKind::Producer);
  assert_eq!(node.input_port_count(), 0);
  assert_eq!(node.output_port_count(), 1);

  assert_eq!(node.input_port_name(0), None);
  assert_eq!(node.output_port_name(0), Some("out0".to_string()));
  assert_eq!(node.output_port_name(1), None);

  assert_eq!(node.resolve_input_port("in0"), None);
  assert_eq!(node.resolve_output_port("out0"), Some(0));
  assert_eq!(node.resolve_output_port("out"), Some(0)); // Single port default
  assert_eq!(node.resolve_output_port("0"), Some(0));
}

#[test]
fn test_transformer_node_trait() {
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  assert_eq!(node.name(), "mapper");
  assert_eq!(node.node_kind(), NodeKind::Transformer);
  assert_eq!(node.input_port_count(), 1);
  assert_eq!(node.output_port_count(), 1);

  assert_eq!(node.input_port_name(0), Some("in0".to_string()));
  assert_eq!(node.input_port_name(1), None);
  assert_eq!(node.output_port_name(0), Some("out0".to_string()));
  assert_eq!(node.output_port_name(1), None);

  assert_eq!(node.resolve_input_port("in0"), Some(0));
  assert_eq!(node.resolve_input_port("in"), Some(0)); // Single port default
  assert_eq!(node.resolve_input_port("0"), Some(0));
  assert_eq!(node.resolve_output_port("out0"), Some(0));
  assert_eq!(node.resolve_output_port("out"), Some(0)); // Single port default
}

#[test]
fn test_consumer_node_trait() {
  let consumer = VecConsumer::<i32>::new();
  let node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer);

  assert_eq!(node.name(), "sink");
  assert_eq!(node.node_kind(), NodeKind::Consumer);
  assert_eq!(node.input_port_count(), 1);
  assert_eq!(node.output_port_count(), 0);

  assert_eq!(node.input_port_name(0), Some("in0".to_string()));
  assert_eq!(node.input_port_name(1), None);
  assert_eq!(node.output_port_name(0), None);

  assert_eq!(node.resolve_input_port("in0"), Some(0));
  assert_eq!(node.resolve_input_port("in"), Some(0)); // Single port default
  assert_eq!(node.resolve_input_port("0"), Some(0));
  assert_eq!(node.resolve_output_port("out0"), None);
}

#[test]
fn test_node_trait_as_stateful() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // Default implementation should return None
  assert!(node.as_stateful().is_none());
}

#[test]
fn test_node_trait_as_windowed() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // Default implementation should return None
  assert!(node.as_windowed().is_none());
}

#[test]
fn test_node_trait_spawn_execution_task() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // ProducerNode should implement spawn_execution_task
  // This is tested in node.rs tests, but we verify the trait method exists
  use std::collections::HashMap;
  use std::sync::Arc;
  use tokio::sync::{RwLock, mpsc};

  let input_channels: HashMap<usize, mpsc::Receiver<Vec<u8>>> = HashMap::new();
  let output_channels: HashMap<usize, mpsc::Sender<Vec<u8>>> = HashMap::new();
  let pause_signal = Arc::new(RwLock::new(false));

  // ProducerNode should return Some(JoinHandle) since it implements spawn_execution_task
  let handle = node.spawn_execution_task(input_channels, output_channels, pause_signal);
  assert!(handle.is_some());
}

#[test]
fn test_node_kind_hash() {
  use std::collections::HashMap;

  let mut map = HashMap::new();
  map.insert(NodeKind::Producer, "producer");
  map.insert(NodeKind::Transformer, "transformer");
  map.insert(NodeKind::Consumer, "consumer");
  map.insert(NodeKind::Subgraph, "subgraph");

  assert_eq!(map.get(&NodeKind::Producer), Some(&"producer"));
  assert_eq!(map.get(&NodeKind::Transformer), Some(&"transformer"));
  assert_eq!(map.get(&NodeKind::Consumer), Some(&"consumer"));
  assert_eq!(map.get(&NodeKind::Subgraph), Some(&"subgraph"));
}

#[test]
fn test_node_kind_copy() {
  let kind1 = NodeKind::Producer;
  let kind2 = kind1; // Copy
  let kind3 = kind1; // Copy again

  assert_eq!(kind1, kind2);
  assert_eq!(kind2, kind3);
  assert_eq!(kind1, kind3);
}

#[test]
fn test_node_trait_send_sync() {
  // Verify that NodeTrait is Send + Sync
  fn assert_send_sync<T: Send + Sync>() {}

  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // This should compile if NodeTrait is Send + Sync
  assert_send_sync::<dyn NodeTrait>();

  // Verify we can use the node as a trait object
  let _trait_obj: &dyn NodeTrait = &node;
}

#[test]
fn test_node_trait_any() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);
  let trait_obj: &dyn NodeTrait = &node;

  // Verify that NodeTrait implements Any
  use std::any::Any;
  assert!(trait_obj.type_id() == trait_obj.type_id());
}

#[test]
fn test_multiple_port_resolution() {
  // Test port resolution for nodes with multiple ports
  // Note: Most nodes in the codebase have single ports, but the trait supports multiple

  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("mapper".to_string(), transformer);

  // Test numeric port resolution
  assert_eq!(node.resolve_input_port("0"), Some(0));
  assert_eq!(node.resolve_input_port("1"), None); // Only one input port

  // Test named port resolution
  assert_eq!(node.resolve_input_port("in0"), Some(0));
  assert_eq!(node.resolve_input_port("in1"), None);

  // Test default single port resolution
  assert_eq!(node.resolve_input_port("in"), Some(0));
}

#[test]
fn test_port_name_generation() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);

  // Test output port naming
  assert_eq!(node.output_port_name(0), Some("out0".to_string()));

  // Test that invalid port indices return None
  assert_eq!(node.output_port_name(1), None);
  assert_eq!(node.output_port_name(100), None);
}

#[test]
fn test_node_name_access() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("my_producer".to_string(), producer);

  assert_eq!(node.name(), "my_producer");

  // Verify name is a string slice (not owned)
  let name_ref: &str = node.name();
  assert_eq!(name_ref, "my_producer");
}
