//! # Graph Structure
//!
//! This module provides the core `Graph` type for managing graph-based data processing pipelines.
//!
//! ## Graph Structure
//!
//! A `Graph` contains nodes and connections between them. Nodes can be producers,
//! transformers, or consumers, and connections define how data flows between nodes.

use std::collections::HashMap;

use super::traits::NodeTrait;

/// Runtime connection information for graph execution.
///
/// This structure represents a connection between two nodes at runtime,
/// using node names and port names (not compile-time types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
  /// Source node name and output port name
  pub source: (String, String),
  /// Target node name and input port name
  pub target: (String, String),
}

/// A graph structure containing nodes and their connections.
///
/// The graph manages type-erased nodes and their runtime connections.
/// All data flowing through the graph is wrapped in `Message<T>` for
/// traceability and metadata preservation.
pub struct Graph {
  /// Map of node names to type-erased nodes
  nodes: HashMap<String, Box<dyn NodeTrait + Send + Sync>>,
  /// List of connections between nodes
  connections: Vec<ConnectionInfo>,
}

impl Graph {
  /// Creates a new empty graph.
  ///
  /// # Returns
  ///
  /// A new `Graph` instance with no nodes or connections.
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      connections: Vec::new(),
    }
  }

  /// Returns the number of nodes in the graph.
  ///
  /// # Returns
  ///
  /// The number of nodes in the graph.
  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }

  /// Checks if the graph is empty (has no nodes).
  ///
  /// # Returns
  ///
  /// `true` if the graph has no nodes, `false` otherwise.
  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }

  /// Gets a node by name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to retrieve
  ///
  /// # Returns
  ///
  /// `Some(&dyn NodeTrait)` if the node exists, `None` otherwise.
  pub fn get_node(&self, name: &str) -> Option<&dyn NodeTrait> {
    // Return as &dyn NodeTrait (without Send + Sync bounds in return type)
    // The stored nodes have Send + Sync bounds, but we can safely return them as &dyn NodeTrait
    self.nodes.get(name).map(|n| n.as_ref() as &dyn NodeTrait)
  }

  /// Gets a mutable reference to a node by name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to retrieve
  ///
  /// # Returns
  ///
  /// `Some(&mut dyn NodeTrait)` if the node exists, `None` otherwise.
  pub fn get_node_mut(&mut self, name: &str) -> Option<&mut dyn NodeTrait> {
    // Return as &mut dyn NodeTrait (without Send + Sync bounds in return type)
    // The stored nodes have Send + Sync bounds, but we can safely return them as &mut dyn NodeTrait
    self
      .nodes
      .get_mut(name)
      .map(|n| n.as_mut() as &mut dyn NodeTrait)
  }

  /// Returns a vector of all node names in the graph.
  ///
  /// # Returns
  ///
  /// A vector of node names, in no particular order.
  pub fn node_names(&self) -> Vec<String> {
    self.nodes.keys().cloned().collect()
  }

  /// Gets all connections in the graph.
  ///
  /// # Returns
  ///
  /// A reference to the list of connections.
  pub fn get_connections(&self) -> &[ConnectionInfo] {
    &self.connections
  }

  /// Gets all connections in the graph (mutable).
  ///
  /// # Returns
  ///
  /// A mutable reference to the list of connections.
  pub fn get_connections_mut(&mut self) -> &mut Vec<ConnectionInfo> {
    &mut self.connections
  }

  /// Gets the parent nodes and their output ports for a given node.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  ///
  /// # Returns
  ///
  /// A vector of `(parent_node_name, parent_output_port_name)` tuples.
  pub fn get_parents(&self, node_name: &str) -> Vec<(String, String)> {
    self
      .connections
      .iter()
      .filter(|conn| conn.target.0 == node_name)
      .map(|conn| (conn.source.0.clone(), conn.source.1.clone()))
      .collect()
  }

  /// Gets the child nodes and their input ports for a given node.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  ///
  /// # Returns
  ///
  /// A vector of `(child_node_name, child_input_port_name)` tuples.
  pub fn get_children(&self, node_name: &str) -> Vec<(String, String)> {
    self
      .connections
      .iter()
      .filter(|conn| conn.source.0 == node_name)
      .map(|conn| (conn.target.0.clone(), conn.target.1.clone()))
      .collect()
  }

  /// Adds a node to the graph.
  ///
  /// # Arguments
  ///
  /// * `node` - A boxed node trait object
  ///
  /// # Returns
  ///
  /// `Ok(())` if the node was added successfully, or an error if a node with
  /// the same name already exists.
  pub fn add_node(&mut self, node: Box<dyn NodeTrait + Send + Sync>) -> Result<(), String> {
    let name = node.name().to_string();
    if self.nodes.contains_key(&name) {
      return Err(format!("Node with name '{}' already exists", name));
    }
    self.nodes.insert(name, node);
    Ok(())
  }

  /// Connects two nodes by name using their default ports.
  ///
  /// # Arguments
  ///
  /// * `source_name` - The name of the source node
  /// * `target_name` - The name of the target node
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was created successfully, or an error if
  /// the nodes don't exist or don't have the required ports.
  pub fn connect_by_name(&mut self, source_name: &str, target_name: &str) -> Result<(), String> {
    // Get source and target nodes
    let source_node = self
      .get_node(source_name)
      .ok_or_else(|| format!("Source node '{}' not found", source_name))?;
    let target_node = self
      .get_node(target_name)
      .ok_or_else(|| format!("Target node '{}' not found", target_name))?;

    // Get default ports (first output port of source, first input port of target)
    let source_ports = source_node.output_port_names();
    let target_ports = target_node.input_port_names();

    if source_ports.is_empty() {
      return Err(format!("Source node '{}' has no output ports", source_name));
    }
    if target_ports.is_empty() {
      return Err(format!("Target node '{}' has no input ports", target_name));
    }

    let source_port = source_ports[0].clone();
    let target_port = target_ports[0].clone();

    // Verify ports exist
    if !source_node.has_output_port(&source_port) {
      return Err(format!(
        "Source node '{}' does not have output port '{}'",
        source_name, source_port
      ));
    }
    if !target_node.has_input_port(&target_port) {
      return Err(format!(
        "Target node '{}' does not have input port '{}'",
        target_name, target_port
      ));
    }

    // Create connection
    self.connections.push(ConnectionInfo {
      source: (source_name.to_string(), source_port),
      target: (target_name.to_string(), target_port),
    });

    Ok(())
  }

  /// Connects two nodes by name and port names.
  ///
  /// # Arguments
  ///
  /// * `source_name` - The name of the source node
  /// * `source_port` - The name of the source output port
  /// * `target_name` - The name of the target node
  /// * `target_port` - The name of the target input port
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was created successfully, or an error if
  /// the nodes don't exist or don't have the required ports.
  pub fn connect(
    &mut self,
    source_name: &str,
    source_port: &str,
    target_name: &str,
    target_port: &str,
  ) -> Result<(), String> {
    // Get source and target nodes
    let source_node = self
      .get_node(source_name)
      .ok_or_else(|| format!("Source node '{}' not found", source_name))?;
    let target_node = self
      .get_node(target_name)
      .ok_or_else(|| format!("Target node '{}' not found", target_name))?;

    // Verify ports exist
    if !source_node.has_output_port(source_port) {
      return Err(format!(
        "Source node '{}' does not have output port '{}'",
        source_name, source_port
      ));
    }
    if !target_node.has_input_port(target_port) {
      return Err(format!(
        "Target node '{}' does not have input port '{}'",
        target_name, target_port
      ));
    }

    // Create connection
    self.connections.push(ConnectionInfo {
      source: (source_name.to_string(), source_port.to_string()),
      target: (target_name.to_string(), target_port.to_string()),
    });

    Ok(())
  }
}

impl Clone for Graph {
  fn clone(&self) -> Self {
    // Note: We cannot clone Box<dyn NodeTrait> directly since Clone is not object-safe
    // For now, we create a new graph with the same structure.
    // In practice, nodes should be cloned before adding to the graph if needed.
    // This is a limitation of Rust's trait object system.
    Self {
      nodes: HashMap::new(), // Nodes must be re-added after cloning
      connections: self.connections.clone(),
    }
  }
}

impl Default for Graph {
  fn default() -> Self {
    Self::new()
  }
}
