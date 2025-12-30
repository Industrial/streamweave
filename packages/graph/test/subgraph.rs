//! Subgraph tests
//!
//! This module provides integration tests for subgraph functionality.

use streamweave_graph::{Graph, GraphBuilder, NodeKind, ProducerNode, SubgraphNode};
use streamweave_vec::VecProducer;

#[test]
fn test_subgraph_node_creation() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new("subgraph".to_string(), graph, 2, 1);

  assert_eq!(subgraph.name(), "subgraph");
  assert_eq!(subgraph.node_kind(), NodeKind::Subgraph);
  assert_eq!(subgraph.input_port_count(), 2);
  assert_eq!(subgraph.output_port_count(), 1);
}

#[test]
fn test_subgraph_node_port_names() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new("subgraph".to_string(), graph, 2, 1);

  assert_eq!(subgraph.input_port_name(0), Some("in0".to_string()));
  assert_eq!(subgraph.input_port_name(1), Some("in1".to_string()));
  assert_eq!(subgraph.input_port_name(2), None);

  assert_eq!(subgraph.output_port_name(0), Some("out0".to_string()));
  assert_eq!(subgraph.output_port_name(1), None);
}

#[test]
fn test_subgraph_node_port_resolution() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new("subgraph".to_string(), graph, 1, 1);

  assert_eq!(subgraph.resolve_input_port("0"), Some(0));
  assert_eq!(subgraph.resolve_input_port("in0"), Some(0));
  assert_eq!(subgraph.resolve_input_port("in"), Some(0)); // Single port default
  assert_eq!(subgraph.resolve_input_port("invalid"), None);

  assert_eq!(subgraph.resolve_output_port("0"), Some(0));
  assert_eq!(subgraph.resolve_output_port("out0"), Some(0));
  assert_eq!(subgraph.resolve_output_port("out"), Some(0)); // Single port default
  assert_eq!(subgraph.resolve_output_port("invalid"), None);
}

#[test]
fn test_subgraph_node_port_mapping() {
  // Create a simple graph with one node
  let inner_graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();

  let mut subgraph = SubgraphNode::new("subgraph".to_string(), inner_graph, 0, 1);

  // Map output port 0 to the internal "source" node's output port 0
  assert!(subgraph.map_output_port(0, "source".to_string(), 0).is_ok());

  // Verify the mapping
  assert_eq!(
    subgraph.get_output_port_mapping(0),
    Some(&("source".to_string(), 0))
  );

  // Invalid port mapping should fail
  assert!(
    subgraph
      .map_output_port(1, "source".to_string(), 0)
      .is_err()
  ); // Port 1 doesn't exist
  assert!(
    subgraph
      .map_output_port(0, "nonexistent".to_string(), 0)
      .is_err()
  ); // Node doesn't exist
}

#[test]
fn test_subgraph_in_graph() {
  // Create a subgraph
  let inner_graph = Graph::new();
  let subgraph = SubgraphNode::new("subgraph".to_string(), inner_graph, 1, 1);

  // Add subgraph to a graph
  let graph = GraphBuilder::new()
    .add_node("subgraph".to_string(), subgraph)
    .unwrap()
    .build();

  // Verify the subgraph is in the graph
  let node = graph.get_node("subgraph");
  assert!(node.is_some());
  assert_eq!(node.unwrap().node_kind(), NodeKind::Subgraph);
  assert_eq!(node.unwrap().input_port_count(), 1);
  assert_eq!(node.unwrap().output_port_count(), 1);
}

#[test]
fn test_subgraph_graph_access() {
  let inner_graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();

  let mut subgraph = SubgraphNode::new("subgraph".to_string(), inner_graph, 0, 1);

  // Test that we can access the graph
  let _graph_ref = subgraph.graph();
  let _graph_mut = subgraph.graph_mut();
}

#[test]
fn test_subgraph_input_port_mapping() {
  use streamweave_graph::ConsumerNode;
  use streamweave_vec::VecConsumer;

  // Create a simple graph with a consumer node
  let inner_graph = GraphBuilder::new()
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))
    .unwrap()
    .build();

  let mut subgraph = SubgraphNode::new("subgraph".to_string(), inner_graph, 1, 0);

  // Map input port 0 to the internal "sink" node's input port 0
  assert!(subgraph.map_input_port(0, "sink".to_string(), 0).is_ok());

  // Verify the mapping
  assert_eq!(
    subgraph.get_input_port_mapping(0),
    Some(&("sink".to_string(), 0))
  );

  // Invalid port mapping should fail
  assert!(subgraph.map_input_port(1, "sink".to_string(), 0).is_err()); // Port 1 doesn't exist
  assert!(
    subgraph
      .map_input_port(0, "nonexistent".to_string(), 0)
      .is_err()
  ); // Node doesn't exist
}

#[test]
fn test_subgraph_port_mapping_validation() {
  let inner_graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();

  let mut subgraph = SubgraphNode::new("subgraph".to_string(), inner_graph, 0, 1);

  // Try to map to invalid internal port
  assert!(
    subgraph
      .map_output_port(0, "source".to_string(), 1)
      .is_err()
  ); // Port 1 doesn't exist on source node
}
