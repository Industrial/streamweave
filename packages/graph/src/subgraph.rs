//! # Subgraph Support
//!
//! This module provides support for using graphs as nodes in other graphs.
//! This enables hierarchical graph composition and modular graph design.

use crate::graph::Graph;
use crate::traits::{NodeKind, NodeTrait};
use std::collections::HashMap;

/// A node that wraps a Graph, allowing it to be used as a node in another graph.
///
/// SubgraphNode enables hierarchical graph composition by treating a complete
/// graph as a single node with input and output ports. The subgraph's ports
/// are mapped to internal nodes within the subgraph.
///
/// # Port Mapping
///
/// The subgraph's input/output ports are mapped to internal nodes:
/// - Input ports map to consumer nodes or transformer input ports within the subgraph
/// - Output ports map to producer nodes or transformer output ports within the subgraph
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::{Graph, SubgraphNode};
///
/// // Create a subgraph
/// let subgraph = Graph::new();
///
/// // Create a subgraph node with 1 input and 1 output port
/// let subgraph_node = SubgraphNode::new(
///     "my_subgraph".to_string(),
///     subgraph,
///     1, // 1 input port
///     1, // 1 output port
/// );
/// ```
pub struct SubgraphNode {
  /// The name of this subgraph node
  name: String,
  /// The graph contained within this subgraph
  graph: Graph,
  /// Number of input ports
  input_port_count: usize,
  /// Number of output ports
  output_port_count: usize,
  /// Mapping from subgraph input port index to internal node name and port
  /// Key: subgraph input port index
  /// Value: (internal_node_name, internal_port_index)
  input_port_map: HashMap<usize, (String, usize)>,
  /// Mapping from subgraph output port index to internal node name and port
  /// Key: subgraph output port index
  /// Value: (internal_node_name, internal_port_index)
  output_port_map: HashMap<usize, (String, usize)>,
}

impl SubgraphNode {
  /// Creates a new SubgraphNode with the given name, graph, and port counts.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this subgraph node
  /// * `graph` - The graph to wrap as a subgraph
  /// * `input_port_count` - The number of input ports this subgraph exposes
  /// * `output_port_count` - The number of output ports this subgraph exposes
  ///
  /// # Returns
  ///
  /// A new `SubgraphNode` instance with empty port mappings.
  ///
  /// # Note
  ///
  /// After creating a SubgraphNode, you must map the ports to internal nodes
  /// using `map_input_port()` and `map_output_port()`.
  pub fn new(
    name: String,
    graph: Graph,
    input_port_count: usize,
    output_port_count: usize,
  ) -> Self {
    Self {
      name,
      graph,
      input_port_count,
      output_port_count,
      input_port_map: HashMap::new(),
      output_port_map: HashMap::new(),
    }
  }

  /// Maps a subgraph input port to an internal node and port.
  ///
  /// # Arguments
  ///
  /// * `subgraph_port` - The subgraph input port index
  /// * `internal_node` - The name of the internal node to connect to
  /// * `internal_port` - The port index on the internal node
  ///
  /// # Returns
  ///
  /// `Ok(())` if the mapping was successful, `Err(String)` if the internal
  /// node doesn't exist or the port is invalid.
  pub fn map_input_port(
    &mut self,
    subgraph_port: usize,
    internal_node: String,
    internal_port: usize,
  ) -> Result<(), String> {
    if subgraph_port >= self.input_port_count {
      return Err(format!(
        "Subgraph input port {} is out of range (0..{})",
        subgraph_port, self.input_port_count
      ));
    }

    // Validate internal node exists
    if let Some(node) = self.graph.get_node(&internal_node) {
      if internal_port >= node.input_port_count() {
        return Err(format!(
          "Internal node '{}' does not have input port {}",
          internal_node, internal_port
        ));
      }
    } else {
      return Err(format!("Internal node '{}' does not exist", internal_node));
    }

    self
      .input_port_map
      .insert(subgraph_port, (internal_node, internal_port));
    Ok(())
  }

  /// Maps a subgraph output port to an internal node and port.
  ///
  /// # Arguments
  ///
  /// * `subgraph_port` - The subgraph output port index
  /// * `internal_node` - The name of the internal node to connect from
  /// * `internal_port` - The port index on the internal node
  ///
  /// # Returns
  ///
  /// `Ok(())` if the mapping was successful, `Err(String)` if the internal
  /// node doesn't exist or the port is invalid.
  pub fn map_output_port(
    &mut self,
    subgraph_port: usize,
    internal_node: String,
    internal_port: usize,
  ) -> Result<(), String> {
    if subgraph_port >= self.output_port_count {
      return Err(format!(
        "Subgraph output port {} is out of range (0..{})",
        subgraph_port, self.output_port_count
      ));
    }

    // Validate internal node exists
    if let Some(node) = self.graph.get_node(&internal_node) {
      if internal_port >= node.output_port_count() {
        return Err(format!(
          "Internal node '{}' does not have output port {}",
          internal_node, internal_port
        ));
      }
    } else {
      return Err(format!("Internal node '{}' does not exist", internal_node));
    }

    self
      .output_port_map
      .insert(subgraph_port, (internal_node, internal_port));
    Ok(())
  }

  /// Returns a reference to the wrapped graph.
  ///
  /// # Returns
  ///
  /// A reference to the `Graph` contained in this subgraph.
  pub fn graph(&self) -> &Graph {
    &self.graph
  }

  /// Returns a mutable reference to the wrapped graph.
  ///
  /// # Returns
  ///
  /// A mutable reference to the `Graph` contained in this subgraph.
  pub fn graph_mut(&mut self) -> &mut Graph {
    &mut self.graph
  }

  /// Returns the input port mapping for a given port index.
  ///
  /// # Arguments
  ///
  /// * `port_index` - The subgraph input port index
  ///
  /// # Returns
  ///
  /// `Some((node_name, port_index))` if the port is mapped, `None` otherwise.
  pub fn get_input_port_mapping(&self, port_index: usize) -> Option<&(String, usize)> {
    self.input_port_map.get(&port_index)
  }

  /// Returns the output port mapping for a given port index.
  ///
  /// # Arguments
  ///
  /// * `port_index` - The subgraph output port index
  ///
  /// # Returns
  ///
  /// `Some((node_name, port_index))` if the port is mapped, `None` otherwise.
  pub fn get_output_port_mapping(&self, port_index: usize) -> Option<&(String, usize)> {
    self.output_port_map.get(&port_index)
  }
}

impl NodeTrait for SubgraphNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Subgraph
  }

  fn input_port_count(&self) -> usize {
    self.input_port_count
  }

  fn output_port_count(&self) -> usize {
    self.output_port_count
  }

  fn input_port_name(&self, index: usize) -> Option<String> {
    if index < self.input_port_count {
      Some(format!("in{}", index))
    } else {
      None
    }
  }

  fn output_port_name(&self, index: usize) -> Option<String> {
    if index < self.output_port_count {
      Some(format!("out{}", index))
    } else {
      None
    }
  }

  fn resolve_input_port(&self, port_name: &str) -> Option<usize> {
    // Try numeric index first
    if let Ok(index) = port_name.parse::<usize>()
      && index < self.input_port_count
    {
      return Some(index);
    }

    // Try "in0", "in1", etc.
    if let Some(stripped) = port_name.strip_prefix("in")
      && let Ok(index) = stripped.parse::<usize>()
      && index < self.input_port_count
    {
      return Some(index);
    }

    // Default to "in" for single-port subgraphs
    if port_name == "in" && self.input_port_count == 1 {
      return Some(0);
    }

    None
  }

  fn resolve_output_port(&self, port_name: &str) -> Option<usize> {
    // Try numeric index first
    if let Ok(index) = port_name.parse::<usize>()
      && index < self.output_port_count
    {
      return Some(index);
    }

    // Try "out0", "out1", etc.
    if let Some(stripped) = port_name.strip_prefix("out")
      && let Ok(index) = stripped.parse::<usize>()
      && index < self.output_port_count
    {
      return Some(index);
    }

    // Default to "out" for single-port subgraphs
    if port_name == "out" && self.output_port_count == 1 {
      return Some(0);
    }

    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
    use crate::{GraphBuilder, ProducerNode};
    use streamweave_vec::VecProducer;

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
    use crate::GraphBuilder;

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
}
