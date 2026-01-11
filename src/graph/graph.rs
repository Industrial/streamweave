//! # Graph Structure
//!
//! This module provides the core `Graph` type for managing graph-based data processing pipelines.
//! It defines the runtime structure that holds nodes and their connections, enabling complex
//! data processing topologies with fan-in, fan-out, and multi-path data flows.
//!
//! # Overview
//!
//! The [`Graph`] type represents a complete data processing topology with nodes and connections.
//! It provides runtime management of type-erased nodes, allowing graphs to be constructed
//! dynamically and executed by the execution engine. All data flowing through the graph is
//! automatically wrapped in `Message<T>` for traceability and metadata preservation.
//!
//! # Key Concepts
//!
//! - **Node Management**: Stores type-erased nodes in a map for runtime access
//! - **Connection Tracking**: Maintains a list of connections between nodes
//! - **Type Erasure**: Uses `Box<dyn NodeTrait>` to enable runtime graph construction
//! - **Message-Based Flow**: All data flows as `Message<T>` with automatic wrapping/unwrapping
//! - **Graph Execution**: Works with the execution engine to run the graph topology
//!
//! # Core Types
//!
//! - **[`Graph`]**: Runtime graph structure containing nodes and connections
//! - **[`ConnectionInfo`]**: Runtime connection information between nodes
//!
//! # Quick Start
//!
//! ## Building a Graph
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, GraphExecution};
//! use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave_array::ArrayProducer;
//! use streamweave::transformers::MapTransformer;
//! use streamweave::consumers::VecConsumer;
//!
//! // Build a graph using GraphBuilder
//! let graph = GraphBuilder::new()
//!     .node(ProducerNode::from_producer(
//!         "source".to_string(),
//!         ArrayProducer::new([1, 2, 3]),
//!     ))?
//!     .node(TransformerNode::from_transformer(
//!         "double".to_string(),
//!         MapTransformer::new(|x: i32| x * 2),
//!     ))?
//!     .node(ConsumerNode::from_consumer(
//!         "sink".to_string(),
//!         VecConsumer::<i32>::new(),
//!     ))?
//!     .connect_by_name("source", "double")?
//!     .connect_by_name("double", "sink")?
//!     .build();
//!
//! // Execute the graph
//! let mut executor = graph.executor();
//! executor.start().await?;
//! ```
//!
//! ## Graph Structure
//!
//! A [`Graph`] contains:
//! - **Nodes**: Type-erased nodes stored by name in a `HashMap`
//! - **Connections**: List of `ConnectionInfo` representing data flow between nodes
//!
//! Nodes can be producers, transformers, or consumers, and connections define how
//! data flows between nodes. All data is automatically wrapped in `Message<T>`
//! during execution.
//!
//! # Design Decisions
//!
//! - **Type Erasure**: Uses `Box<dyn NodeTrait>` to enable runtime graph construction
//!   and dynamic topology management
//! - **Name-Based Access**: Nodes are accessed by name for runtime flexibility
//! - **Connection List**: Maintains a list of connections for execution engine setup
//! - **Message-Based**: All data flows as `Message<T>` with automatic handling
//!
//! # Integration with StreamWeave
//!
//! [`Graph`] is typically constructed using [`crate::graph::GraphBuilder`] for compile-time type
//! validation, then executed using [`crate::graph::GraphExecution`]. The execution engine handles
//! concurrent node execution and message routing through channels.

// Import for rustdoc links
#[allow(unused_imports)]
use super::execution::GraphExecution;
#[allow(unused_imports)]
use super::graph_builder::GraphBuilder;

use std::collections::HashMap;

use super::traits::NodeTrait;
use tracing::{debug, trace};

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
    trace!("Graph::new()");
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
    trace!("Graph::node_count()");
    self.nodes.len()
  }

  /// Checks if the graph is empty (has no nodes).
  ///
  /// # Returns
  ///
  /// `true` if the graph has no nodes, `false` otherwise.
  pub fn is_empty(&self) -> bool {
    trace!("Graph::is_empty()");
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
    trace!("Graph::get_node(name = {})", name);
    // Return as &dyn NodeTrait (without Send + Sync bounds in return type)
    // The stored nodes have Send + Sync bounds, but we can safely return them as &dyn NodeTrait
    let result = self.nodes.get(name).map(|n| n.as_ref() as &dyn NodeTrait);
    if result.is_none() {
      debug!(node = %name, "Graph: Node not found");
    }
    result
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
    trace!("Graph::get_node_mut(name = {})", name);
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
    trace!("Graph::node_names()");
    self.nodes.keys().cloned().collect()
  }

  /// Gets all connections in the graph.
  ///
  /// # Returns
  ///
  /// A reference to the list of connections.
  pub fn get_connections(&self) -> &[ConnectionInfo] {
    trace!("Graph::get_connections()");
    &self.connections
  }

  /// Gets all connections in the graph (mutable).
  ///
  /// # Returns
  ///
  /// A mutable reference to the list of connections.
  pub fn get_connections_mut(&mut self) -> &mut Vec<ConnectionInfo> {
    trace!("Graph::get_connections_mut()");
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
    trace!("Graph::get_parents(node_name = {})", node_name);
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
    trace!("Graph::get_children(node_name = {})", node_name);
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
    trace!("Graph::add_node()");
    let name = node.name().to_string();
    if self.nodes.contains_key(&name) {
      return Err(format!("Node with name '{}' already exists", name));
    }
    debug!(node = %name, "Graph: Adding node");
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
    trace!(
      "Graph::connect_by_name(source_name = {}, target_name = {})",
      source_name, target_name
    );

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
      source: (source_name.to_string(), source_port.clone()),
      target: (target_name.to_string(), target_port.clone()),
    });
    debug!(
      source = %source_name,
      source_port = %source_port,
      target = %target_name,
      target_port = %target_port,
      "Graph: Connected nodes"
    );

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
    trace!(
      "Graph::connect(source_name = {}, source_port = {}, target_name = {}, target_port = {})",
      source_name, source_port, target_name, target_port
    );

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
    debug!(
      source = %source_name,
      source_port = %source_port,
      target = %target_name,
      target_port = %target_port,
      "Graph: Connected nodes (with ports)"
    );

    Ok(())
  }
}

impl Clone for Graph {
  fn clone(&self) -> Self {
    trace!("Graph::clone()");
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
    trace!("Graph::default()");
    Self::new()
  }
}
