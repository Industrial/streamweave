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

  // Multi-port subgraph uses letter-based names: "in_a", "in_b", etc.
  assert_eq!(subgraph.input_port_name(0), Some("in_a".to_string()));
  assert_eq!(subgraph.input_port_name(1), Some("in_b".to_string()));
  assert_eq!(subgraph.input_port_name(2), None);

  assert_eq!(subgraph.output_port_name(0), Some("out".to_string())); // Single output port uses "out"
  assert_eq!(subgraph.output_port_name(1), None);
}

#[test]
fn test_subgraph_node_port_resolution() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new("subgraph".to_string(), graph, 1, 1);

  // Single-port subgraph uses "in" and "out"
  assert_eq!(subgraph.resolve_input_port("in"), Some(0)); // Single-port uses "in"
  assert_eq!(subgraph.resolve_input_port("0"), None); // Numeric indices no longer supported
  assert_eq!(subgraph.resolve_input_port("in0"), None); // No longer supported
  assert_eq!(subgraph.resolve_input_port("invalid"), None);

  assert_eq!(subgraph.resolve_output_port("out"), Some(0)); // Single-port uses "out"
  assert_eq!(subgraph.resolve_output_port("0"), None); // Numeric indices no longer supported
  assert_eq!(subgraph.resolve_output_port("out0"), None); // No longer supported
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

// Tests moved from src/
#[test]
fn test_subgraph_node_creation() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    graph,
    vec!["in_a".to_string(), "in_b".to_string()],
    vec!["out".to_string()],
  );

  assert_eq!(subgraph.name(), "subgraph");
  assert_eq!(subgraph.node_kind(), NodeKind::Subgraph);
  assert_eq!(subgraph.input_port_names().len(), 2);
  assert_eq!(subgraph.output_port_names().len(), 1);
}

#[test]
fn test_subgraph_node_port_names() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    graph,
    vec!["in_a".to_string(), "in_b".to_string()],
    vec!["out".to_string()],
  );

  // Multi-port subgraph uses letter-based names: "in_a", "in_b", etc.
  assert_eq!(
    subgraph.input_port_names().get(0),
    Some(&"in_a".to_string())
  );
  assert_eq!(
    subgraph.input_port_names().get(1),
    Some(&"in_b".to_string())
  );
  assert_eq!(subgraph.input_port_names().get(2), None);

  // Single output port uses "out"
  assert_eq!(
    subgraph.output_port_names().get(0),
    Some(&"out".to_string())
  );
  assert_eq!(subgraph.output_port_names().get(1), None);
}

#[test]
fn test_subgraph_node_port_resolution() {
  let graph = Graph::new();
  let subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    graph,
    vec!["in".to_string()],
    vec!["out".to_string()],
  );

  // Single-port subgraph uses "in" and "out"
  assert!(subgraph.has_input_port("in")); // Single-port uses "in"
  assert!(!subgraph.has_input_port("0")); // Numeric indices no longer supported
  assert!(!subgraph.has_input_port("in0")); // No longer supported
  assert!(!subgraph.has_input_port("invalid"));

  assert!(subgraph.has_output_port("out")); // Single-port uses "out"
  assert!(!subgraph.has_output_port("0")); // Numeric indices no longer supported
  assert!(!subgraph.has_output_port("out0")); // No longer supported
  assert!(!subgraph.has_output_port("invalid"));
}

#[test]
fn test_subgraph_node_port_mapping() {
  use streamweave_graph::{GraphBuilder, ProducerNode};
  use streamweave_vec::VecProducer;

  // Create a simple graph with one node
  let inner_graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();

  let mut subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    inner_graph,
    vec![],
    vec!["out".to_string()],
  );

  // Map output port "out" to the internal "source" node's output port "out"
  assert!(
    subgraph
      .map_output_port("out", "source".to_string(), "out")
      .is_ok()
  );

  // Verify the mapping
  assert_eq!(
    subgraph.get_output_port_mapping("out"),
    Some(&("source".to_string(), "out".to_string()))
  );

  // Invalid port mapping should fail
  assert!(
    subgraph
      .map_output_port("nonexistent", "source".to_string(), "out")
      .is_err()
  ); // Port doesn't exist
  assert!(
    subgraph
      .map_output_port("out", "nonexistent".to_string(), "out")
      .is_err()
  ); // Node doesn't exist
}

#[test]
fn test_subgraph_in_graph() {
  use streamweave_graph::GraphBuilder;

  // Create a subgraph
  let inner_graph = Graph::new();
  let subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    inner_graph,
    vec!["in".to_string()],
    vec!["out".to_string()],
  );

  // Add subgraph to a graph
  let graph = GraphBuilder::new()
    .add_node("subgraph".to_string(), subgraph)
    .unwrap()
    .build();

  // Verify the subgraph is in the graph
  let node = graph.get_node("subgraph");
  assert!(node.is_some());
  assert_eq!(node.unwrap().node_kind(), NodeKind::Subgraph);
  assert_eq!(node.unwrap().input_port_names().len(), 1);
  assert_eq!(node.unwrap().output_port_names().len(), 1);
}

#[test]
fn test_parameter_port_marking() {
  let graph = Graph::new();
  let mut subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    graph,
    vec!["in_a".to_string(), "in_b".to_string()],
    vec!["out".to_string()],
  );

  // Mark port "in_a" as a parameter port
  assert!(subgraph.mark_parameter_port("in_a").is_ok());
  assert!(subgraph.is_parameter_port("in_a"));
  assert!(!subgraph.is_parameter_port("in_b"));

  // Invalid port name should fail
  assert!(subgraph.mark_parameter_port("nonexistent").is_err());
}

#[test]
fn test_return_port_marking() {
  let graph = Graph::new();
  let mut subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    graph,
    vec!["in".to_string()],
    vec!["out_a".to_string(), "out_b".to_string()],
  );

  // Mark port "out_a" as a return port
  assert!(subgraph.mark_return_port("out_a").is_ok());
  assert!(subgraph.is_return_port("out_a"));
  assert!(!subgraph.is_return_port("out_b"));

  // Invalid port name should fail
  assert!(subgraph.mark_return_port("nonexistent").is_err());
}

#[test]
fn test_parameterized_subgraph() {
  let graph = Graph::new();
  let mut subgraph = SubgraphNode::new(
    "subgraph".to_string(),
    graph,
    vec!["param".to_string(), "stream_in".to_string()],
    vec!["return".to_string(), "stream_out".to_string()],
  );

  // Mark port "param" as parameter, "stream_in" as streaming input
  assert!(subgraph.mark_parameter_port("param").is_ok());
  assert!(!subgraph.is_parameter_port("stream_in"));

  // Mark port "return" as return, "stream_out" as streaming output
  assert!(subgraph.mark_return_port("return").is_ok());
  assert!(!subgraph.is_return_port("stream_out"));

  // Verify parameter and return ports
  assert_eq!(subgraph.parameter_ports().len(), 1);
  assert!(subgraph.parameter_ports().contains("param"));
  assert_eq!(subgraph.return_ports().len(), 1);
  assert!(subgraph.return_ports().contains("return"));
}
