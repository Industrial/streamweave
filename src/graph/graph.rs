//! # Graph Trait - Pure Stream Implementation
//!
//! This module defines the `Graph` trait for managing graph structures and executing
//! async node graphs. Graphs contain nodes and edges, and provide methods for both
//! structure management (sync) and execution (async).
//!
//! ## Stream-Based Execution
//!
//! The graph execution engine works purely with streams:
//!
//! 1. Collects input streams for each node from connected upstream nodes
//! 2. Calls `node.execute(inputs)` which returns output streams
//! 3. Connects output streams to downstream nodes' input streams
//! 4. Drives all streams to completion
//!
//! Channels are used internally for backpressure, but are never exposed to nodes.
//! Nodes only see and work with streams.

use crate::graph::edge::Edge;
use crate::graph::node::Node;
use std::collections::{HashMap, VecDeque};

/// Error type for graph execution operations.
pub type GraphExecutionError = Box<dyn std::error::Error + Send + Sync>;

/// A graph containing nodes and edges.
///
/// Graphs represent the structure of a data processing pipeline, with nodes
/// representing processing components and edges representing data flow between them.
///
/// # Graph Structure
///
/// A graph consists of:
///
/// - **Nodes**: Processing components that implement the `Node` trait
/// - **Edges**: Connections between node ports (stream connections)
///
/// # Structure Management (Synchronous)
///
/// The graph provides synchronous methods for managing its structure:
///
/// - Adding and removing nodes
/// - Adding and removing edges
/// - Querying nodes and edges
///
/// These operations are synchronous because they only modify data structures.
///
/// # Execution (Asynchronous)
///
/// The graph provides asynchronous methods for executing the graph:
///
/// - `execute()` - Starts graph execution by connecting streams between nodes
/// - `stop()` - Gracefully stops graph execution
/// - `wait_for_completion()` - Waits for all nodes to complete execution
///
/// Execution is asynchronous and uses pure stream composition - no channels exposed.
///
/// # Object Safety
///
/// This trait is designed to be object-safe, allowing different graph
/// implementations to be used polymorphically.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::graph::Graph;
/// use streamweave::graph::node::Node;
/// use streamweave::graph::edge::Edge;
///
/// // Build graph structure (sync)
/// graph.add_node("source".to_string(), Box::new(source_node))?;
/// graph.add_node("sink".to_string(), Box::new(sink_node))?;
/// graph.add_edge(Edge {
///     source_node: "source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "sink".to_string(),
///     target_port: "in".to_string(),
/// })?;
///
/// // Execute graph (async)
/// graph.execute().await?;
/// graph.wait_for_completion().await?;
/// ```
use async_trait::async_trait;

#[async_trait]
pub trait Graph {
  /// Returns the name of the graph.
  ///
  /// # Returns
  ///
  /// A string slice containing the graph's name.
  fn name(&self) -> &str;

  /// Sets the name of the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for the graph
  fn set_name(&mut self, name: &str);

  /// Returns all nodes in the graph.
  ///
  /// # Returns
  ///
  /// A vector of references to boxed nodes in the graph.
  fn get_nodes(&self) -> Vec<&dyn Node>;

  /// Gets a node by name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to find
  ///
  /// # Returns
  ///
  /// `Some(&Box<dyn Node>)` if a node with the given name exists, `None` otherwise.
  fn find_node_by_name(&self, name: &str) -> Option<&dyn Node>;

  /// Adds a node to the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to the node (should match `node.name()`)
  /// * `node` - The node to add to the graph
  ///
  /// # Returns
  ///
  /// `Ok(())` if the node was added successfully, or an error if a node with
  /// the same name already exists.
  ///
  /// # Errors
  ///
  /// Returns an error string if a node with the given name already exists in the graph.
  fn add_node(&mut self, name: String, node: Box<dyn Node>) -> Result<(), String>;

  /// Removes a node from the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to remove
  ///
  /// # Returns
  ///
  /// `Ok(())` if the node was removed successfully, or an error if the node
  /// doesn't exist or has connected edges.
  ///
  /// # Errors
  ///
  /// Returns an error string if the node doesn't exist or cannot be removed
  /// (e.g., it has connected edges).
  fn remove_node(&mut self, name: &str) -> Result<(), String>;

  /// Returns all edges in the graph.
  ///
  /// # Returns
  ///
  /// A vector of references to all edges in the graph.
  fn get_edges(&self) -> Vec<&Edge>;

  /// Gets an edge by source and target node and port.
  ///
  /// # Arguments
  ///
  /// * `source_node` - The name of the source node
  /// * `source_port` - The name of the source output port
  /// * `target_node` - The name of the target node
  /// * `target_port` - The name of the target input port
  ///
  /// # Returns
  ///
  /// `Some(&Edge)` if an edge matching the given parameters exists, `None` otherwise.
  fn find_edge_by_nodes_and_ports(
    &self,
    source_node: &str,
    source_port: &str,
    target_node: &str,
    target_port: &str,
  ) -> Option<&Edge>;

  /// Adds an edge to the graph.
  ///
  /// # Arguments
  ///
  /// * `edge` - The edge to add to the graph
  ///
  /// # Returns
  ///
  /// `Ok(())` if the edge was added successfully, or an error if the edge is invalid
  /// (e.g., nodes don't exist or ports don't exist).
  ///
  /// # Errors
  ///
  /// Returns an error string if:
  /// - The source or target node doesn't exist
  /// - The source or target port doesn't exist on the respective node
  /// - The edge would create a duplicate connection
  fn add_edge(&mut self, edge: Edge) -> Result<(), String>;

  /// Removes an edge from the graph.
  ///
  /// # Arguments
  ///
  /// * `source_node` - The name of the source node
  /// * `source_port` - The name of the source output port
  /// * `target_node` - The name of the target node
  /// * `target_port` - The name of the target input port
  ///
  /// # Returns
  ///
  /// `Ok(())` if the edge was removed successfully, or an error if the edge
  /// doesn't exist.
  ///
  /// # Errors
  ///
  /// Returns an error string if no edge matching the given parameters exists.
  fn remove_edge(
    &mut self,
    source_node: &str,
    source_port: &str,
    target_node: &str,
    target_port: &str,
  ) -> Result<(), String>;

  // ============================================================================
  // Async Execution Methods
  // ============================================================================

  /// Executes the graph by connecting streams between nodes.
  ///
  /// This method:
  /// 1. Performs topological sort to determine execution order
  /// 2. For each node, collects input streams from upstream nodes
  /// 3. Calls `node.execute(inputs)` which returns output streams
  /// 4. Connects output streams to downstream nodes' input streams
  /// 5. Drives all streams to completion
  ///
  /// Channels are used internally for backpressure, but are never exposed to nodes.
  /// Nodes only see and work with streams.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was started successfully, or an error if:
  /// - Nodes have invalid port configurations
  /// - Streams cannot be created or connected
  /// - Tasks cannot be spawned
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if execution cannot be started.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// // Build graph structure
  /// graph.add_node("source".to_string(), Box::new(source_node))?;
  /// graph.add_node("sink".to_string(), Box::new(sink_node))?;
  /// graph.add_edge(Edge { ... })?;
  ///
  /// // Start execution
  /// graph.execute().await?;
  ///
  /// // Wait for completion
  /// graph.wait_for_completion().await?;
  /// ```
  async fn execute(&self) -> Result<(), GraphExecutionError>;

  /// Gracefully stops graph execution.
  ///
  /// This method signals all nodes to stop processing and waits for them to complete
  /// their current operations. Nodes should check for stop signals and exit cleanly.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was stopped successfully, or an error if stopping failed.
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if execution cannot be stopped gracefully.
  async fn stop(&self) -> Result<(), GraphExecutionError>;

  /// Waits for all nodes in the graph to complete execution.
  ///
  /// This method blocks until all node tasks have finished. Use this after calling
  /// `execute()` to wait for the graph to finish processing.
  ///
  /// # Returns
  ///
  /// `Ok(())` if all nodes completed successfully, or an error if any node failed.
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if any node execution failed or if waiting timed out.
  async fn wait_for_completion(&self) -> Result<(), GraphExecutionError>;
}

/// Helper function to perform topological sort of nodes in a graph.
///
/// Returns nodes in execution order (sources first, sinks last).
/// This ensures that when we execute nodes, all upstream nodes have already
/// produced their output streams.
pub fn topological_sort(
  nodes: &[&dyn Node],
  edges: &[&Edge],
) -> Result<Vec<String>, GraphExecutionError> {
  let mut in_degree: HashMap<String, usize> = HashMap::new();
  let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

  // Initialize in-degree for all nodes
  for node in nodes {
    in_degree.insert(node.name().to_string(), 0);
    adjacency.insert(node.name().to_string(), Vec::new());
  }

  // Build adjacency list and calculate in-degrees
  for edge in edges {
    let source = edge.source_node().to_string();
    let target = edge.target_node().to_string();

    adjacency.get_mut(&source).unwrap().push(target.clone());
    *in_degree.get_mut(&target).unwrap() += 1;
  }

  // Kahn's algorithm for topological sort
  let mut queue: VecDeque<String> = VecDeque::new();
  for (node_name, &degree) in &in_degree {
    if degree == 0 {
      queue.push_back(node_name.clone());
    }
  }

  let mut result = Vec::new();
  while let Some(node_name) = queue.pop_front() {
    result.push(node_name.clone());

    if let Some(neighbors) = adjacency.get(&node_name) {
      for neighbor in neighbors {
        let degree = in_degree.get_mut(neighbor).unwrap();
        *degree -= 1;
        if *degree == 0 {
          queue.push_back(neighbor.clone());
        }
      }
    }
  }

  // Check for cycles
  if result.len() != nodes.len() {
    return Err("Graph contains cycles".into());
  }

  Ok(result)
}
