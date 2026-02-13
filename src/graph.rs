//! # Graph - Pure Stream Implementation
//!
//! This module defines the `Graph` struct for managing graph structures and executing
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
//!
//! ## Graph as Node (Nested Graphs)
//!
//! `Graph` implements the `Node` trait, allowing graphs to be used as nodes within
//! other graphs. This enables hierarchical composition and reusable subgraphs.
//!
//! ### Port Mapping
//!
//! When a graph is used as a node, you must explicitly map internal node ports to
//! external ports using `expose_input_port()` and `expose_output_port()`:
//!
//! - **Input ports**: `"configuration"` and `"input"` (fixed external names)
//! - **Output ports**: `"output"` and `"error"` (fixed external names)
//!
//! ### Pull-Based Execution
//!
//! Graphs use a pull-based execution model:
//!
//! 1. When a graph's output port is consumed, it signals readiness backward
//! 2. This propagates through internal nodes to the graph's input ports
//! 3. Data flows only when downstream nodes are ready to consume
//!
//! ### Lifecycle Control
//!
//! Graphs support full lifecycle control:
//!
//! - `start()` - Begin execution (sets state to running)
//! - `pause()` - Pause execution (maintains state, stops processing new data)
//! - `resume()` - Resume execution after pause
//! - `stop()` - Stop execution and clear all state (discards in-flight data)
//!
//! ### Example: Nested Graphs
//!
//! ```rust,no_run
//! use streamweave;
//! use streamweave::node::Node;
//! use streamweave::edge::Edge;
//!
//! // Create a subgraph
//! let mut subgraph = Graph::new("subgraph".to_string());
//! // ... add nodes and edges to subgraph ...
//!
//! // Expose internal ports as external ports
//! subgraph.expose_input_port("internal_source", "in", "input")?;
//! subgraph.expose_output_port("internal_sink", "out", "output")?;
//!
//! // Use subgraph as a node in parent graph
//! let mut parent = Graph::new("parent".to_string());
//! let subgraph_node: Box<dyn Node> = Box::new(subgraph);
//! parent.add_node("subgraph".to_string(), subgraph_node)?;
//!
//! // Connect to subgraph's external ports
//! parent.add_edge(Edge {
//!     source_node: "source".to_string(),
//!     source_port: "out".to_string(),
//!     target_node: "subgraph".to_string(),
//!     target_port: "input".to_string(),
//! })?;
//! ```

use crate::edge::Edge;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use async_trait::async_trait;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

/// Default channel capacity for dataflow edges (backpressure).
const DATAFLOW_CHANNEL_CAPACITY: usize = 64;

/// Error type for graph execution operations.
pub type GraphExecutionError = Box<dyn std::error::Error + Send + Sync>;

/// Type alias for execution handles to reduce type complexity
type ExecutionHandleVec = Arc<Mutex<Vec<JoinHandle<Result<(), GraphExecutionError>>>>>;

/// Type alias for the map of nodes restored after dataflow run (reduces type complexity).
type NodesRestoredMap = Arc<Mutex<HashMap<String, Box<dyn Node>>>>;

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
/// - **Port Mappings**: For graphs used as nodes, mappings from internal to external ports
///
/// # Structure Management (Synchronous)
///
/// The graph provides synchronous methods for managing its structure:
///
/// - Adding and removing nodes
/// - Adding and removing edges
/// - Querying nodes and edges
/// - Exposing internal ports as external ports (for nested graphs)
///
/// These operations are synchronous because they only modify data structures.
///
/// # Execution (Asynchronous)
///
/// The graph provides asynchronous methods for executing the graph:
///
/// - `execute()` - Starts graph execution by connecting streams between nodes
/// - `start()` - Sets execution state to running (for pull-based execution)
/// - `pause()` - Pauses execution (maintains state)
/// - `resume()` - Resumes execution after pause
/// - `stop()` - Stops execution and clears all state
/// - `wait_for_completion()` - Waits for all nodes to complete execution
///
/// Execution is asynchronous and uses pure stream composition - no channels exposed.
///
/// # Graph as Node
///
/// `Graph` implements the `Node` trait, allowing graphs to be nested within other graphs.
/// When used as a node, a graph has fixed external ports:
///
/// - **Input ports**: `"configuration"`, `"input"`
/// - **Output ports**: `"output"`, `"error"`
///
/// Use `expose_input_port()` and `expose_output_port()` to map internal node ports
/// to these external ports.
///
/// # Example: Basic Graph
///
/// ```rust,no_run
/// use streamweave::graph::Graph;
/// use streamweave::node::Node;
/// use streamweave::edge::Edge;
/// use streamweave::nodes::variable_node::VariableNode;
///
/// let mut graph = Graph::new("my_graph".to_string());
/// graph.add_node("source".to_string(), Box::new(VariableNode::new("source".to_string()))).unwrap();
/// graph.add_node("sink".to_string(), Box::new(VariableNode::new("sink".to_string()))).unwrap();
/// graph.add_edge(Edge {
///     source_node: "source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "sink".to_string(),
///     target_port: "value".to_string(),
/// }).unwrap();
/// // Then: graph.execute().await?; graph.wait_for_completion().await?;
/// ```
///
/// # Example: Nested Graph (Subgraph)
///
/// ```rust,no_run
/// use streamweave::graph::Graph;
/// use streamweave::node::Node;
/// use streamweave::edge::Edge;
/// use streamweave::nodes::variable_node::VariableNode;
///
/// let mut subgraph = Graph::new("subgraph".to_string());
/// subgraph.add_node("internal_source".to_string(), Box::new(VariableNode::new("internal_source".to_string()))).unwrap();
/// subgraph.add_node("internal_transform".to_string(), Box::new(VariableNode::new("internal_transform".to_string()))).unwrap();
/// subgraph.add_edge(Edge {
///     source_node: "internal_source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "internal_transform".to_string(),
///     target_port: "value".to_string(),
/// }).unwrap();
///
/// subgraph.expose_input_port("internal_source", "value", "input").unwrap();
/// subgraph.expose_output_port("internal_transform", "out", "output").unwrap();
/// let mut parent = Graph::new("parent".to_string());
/// parent.add_node("source".to_string(), Box::new(VariableNode::new("source".to_string()))).unwrap();
/// let subgraph_node: Box<dyn Node> = Box::new(subgraph);
/// parent.add_node("subgraph".to_string(), subgraph_node).unwrap();
/// parent.add_edge(Edge {
///     source_node: "source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "subgraph".to_string(),
///     target_port: "input".to_string(),
/// }).unwrap();
/// ```
/// Port mapping from external port name to internal (node, port).
#[derive(Clone, Debug)]
struct PortMapping {
  /// Internal node name
  node: String,
  /// Internal port name
  port: String,
}

/// A graph containing nodes and edges.
///
/// Graphs represent the structure of a data processing pipeline, with nodes
/// representing processing components and edges representing data flow between them.
///
/// See the module-level documentation for detailed information about graph execution,
/// nested graphs, lifecycle control, and usage examples.
pub struct Graph {
  /// The name of the graph.
  name: String,
  /// Map of node names to node instances. Wrapped for interior mutability so execute_internal can take nodes with &self.
  nodes: Arc<StdMutex<HashMap<String, Box<dyn Node>>>>,
  /// List of edges connecting nodes.
  edges: Vec<Edge>,
  /// Execution handles for spawned node tasks (used for wait_for_completion)
  execution_handles: ExecutionHandleVec,
  /// Stop signal for graceful shutdown
  stop_signal: Arc<tokio::sync::Notify>,
  /// Pause signal for pausing execution
  pause_signal: Arc<tokio::sync::Notify>,
  /// Execution state: 0 = stopped, 1 = running, 2 = paused
  execution_state: Arc<AtomicU8>,
  /// Mapping of external input ports to internal nodes/ports
  /// Key: external port name (e.g., "input") -> (internal_node, internal_port)
  input_port_mapping: HashMap<String, PortMapping>,
  /// Mapping of external output ports to internal nodes/ports
  /// Key: external port name (e.g., "output") -> (internal_node, internal_port)
  output_port_mapping: HashMap<String, PortMapping>,
  /// Connected input channels for external data
  /// Key: external port name -> receiver for input data
  connected_input_channels:
    HashMap<String, Option<tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>>>,
  /// Connected output channels for external data
  /// Key: external port name -> sender for output data
  connected_output_channels: HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
  /// Cached input port names for Node trait
  input_port_names: Vec<String>,
  /// Cached output port names for Node trait
  output_port_names: Vec<String>,
  /// After dataflow execution, nodes are returned here so they can be restored in wait_for_completion.
  nodes_restored_after_run: Option<NodesRestoredMap>,
}

impl Graph {
  /// Creates a new empty graph with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name for the graph
  ///
  /// # Returns
  ///
  /// A new `Graph` instance with no nodes or edges.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::Graph;
  ///
  /// let graph = Graph::new("my_graph".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      name,
      nodes: Arc::new(StdMutex::new(HashMap::new())),
      edges: Vec::new(),
      execution_handles: Arc::new(Mutex::new(Vec::new())),
      stop_signal: Arc::new(tokio::sync::Notify::new()),
      pause_signal: Arc::new(tokio::sync::Notify::new()),
      execution_state: Arc::new(AtomicU8::new(0)), // 0 = stopped
      input_port_mapping: HashMap::new(),
      output_port_mapping: HashMap::new(),
      connected_input_channels: HashMap::new(),
      connected_output_channels: HashMap::new(),
      input_port_names: Vec::new(),
      output_port_names: Vec::new(),
      nodes_restored_after_run: None,
    }
  }

  /// Exposes an internal node's input port as an external input port.
  ///
  /// This allows external streams to flow into the graph through specific internal nodes.
  ///
  /// # Arguments
  ///
  /// * `internal_node` - The name of the internal node
  /// * `internal_port` - The name of the input port on the internal node
  /// * `external_name` - The name of the external port (must be "configuration" or "input")
  ///
  /// # Returns
  ///
  /// `Ok(())` if the port was exposed successfully, or an error if:
  /// - The internal node doesn't exist
  /// - The internal port doesn't exist on the node
  /// - The external port name is invalid (must be "configuration" or "input")
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("source".to_string(), Box::new(VariableNode::new("source".to_string()))).unwrap();
  /// graph.expose_input_port("source", "value", "input").unwrap();
  /// ```
  pub fn expose_input_port(
    &mut self,
    internal_node: &str,
    internal_port: &str,
    external_name: &str,
  ) -> Result<(), String> {
    // Validate internal node exists
    let guard = self.nodes.lock().unwrap();
    let node = guard
      .get(internal_node)
      .ok_or_else(|| format!("Internal node '{}' does not exist", internal_node))?;

    // Validate internal port exists
    if !node.has_input_port(internal_port) {
      return Err(format!(
        "Internal node '{}' does not have input port '{}'",
        internal_node, internal_port
      ));
    }

    // Add mapping
    self.input_port_mapping.insert(
      external_name.to_string(),
      PortMapping {
        node: internal_node.to_string(),
        port: internal_port.to_string(),
      },
    );

    // Add external port name to the list if not already present
    if !self.input_port_names.contains(&external_name.to_string()) {
      self.input_port_names.push(external_name.to_string());
    }

    Ok(())
  }

  /// Exposes an internal node's output port as an external output port.
  ///
  /// This allows internal streams to flow out of the graph through specific internal nodes.
  ///
  /// # Arguments
  ///
  /// * `internal_node` - The name of the internal node
  /// * `internal_port` - The name of the output port on the internal node
  /// * `external_name` - The name of the external port (must be "output" or "error")
  ///
  /// # Returns
  ///
  /// `Ok(())` if the port was exposed successfully, or an error if:
  /// - The internal node doesn't exist
  /// - The internal port doesn't exist on the node
  /// - The external port name is invalid (must be "output" or "error")
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("sink".to_string(), Box::new(VariableNode::new("sink".to_string()))).unwrap();
  /// graph.expose_output_port("sink", "out", "output").unwrap();
  /// ```
  pub fn expose_output_port(
    &mut self,
    internal_node: &str,
    internal_port: &str,
    external_name: &str,
  ) -> Result<(), String> {
    // Validate internal node exists
    let guard = self.nodes.lock().unwrap();
    let node = guard
      .get(internal_node)
      .ok_or_else(|| format!("Internal node '{}' does not exist", internal_node))?;

    // Validate internal port exists
    if !node.has_output_port(internal_port) {
      return Err(format!(
        "Internal node '{}' does not have output port '{}'",
        internal_node, internal_port
      ));
    }

    // Add mapping
    self.output_port_mapping.insert(
      external_name.to_string(),
      PortMapping {
        node: internal_node.to_string(),
        port: internal_port.to_string(),
      },
    );

    // Add external port name to the list if not already present
    if !self.output_port_names.contains(&external_name.to_string()) {
      self.output_port_names.push(external_name.to_string());
    }

    Ok(())
  }

  /// Connects an input channel to an exposed input port.
  ///
  /// This allows external data to be sent to the graph through the specified port.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed input port
  /// * `receiver` - The channel receiver for input data
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was successful, or an error if the port is not exposed.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// use tokio::sync::mpsc;
  /// use std::sync::Arc;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("n".to_string(), Box::new(VariableNode::new("n".to_string()))).unwrap();
  /// graph.expose_input_port("n", "value", "configuration").unwrap();
  /// let (tx, rx) = mpsc::channel(10);
  /// graph.connect_input_channel("configuration", rx).unwrap();
  /// ```
  pub fn connect_input_channel(
    &mut self,
    external_port: &str,
    receiver: tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
  ) -> Result<(), String> {
    if !self.input_port_mapping.contains_key(external_port) {
      return Err(format!(
        "External input port '{}' is not exposed",
        external_port
      ));
    }
    self
      .connected_input_channels
      .insert(external_port.to_string(), Some(receiver));
    Ok(())
  }

  /// Connects an output channel to an exposed output port.
  ///
  /// This allows graph output to be sent to external consumers through the specified port.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed output port
  /// * `sender` - The channel sender for output data
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was successful, or an error if the port is not exposed.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// use tokio::sync::mpsc;
  /// use std::sync::Arc;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("n".to_string(), Box::new(VariableNode::new("n".to_string()))).unwrap();
  /// graph.expose_output_port("n", "out", "output").unwrap();
  /// let (tx, rx) = mpsc::channel(10);
  /// graph.connect_output_channel("output", tx).unwrap();
  /// ```
  pub fn connect_output_channel(
    &mut self,
    external_port: &str,
    sender: tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  ) -> Result<(), String> {
    if !self.output_port_mapping.contains_key(external_port) {
      return Err(format!(
        "External output port '{}' is not exposed",
        external_port
      ));
    }
    self
      .connected_output_channels
      .insert(external_port.to_string(), sender);
    Ok(())
  }

  /// Returns the name of the graph.
  ///
  /// # Returns
  ///
  /// A string slice containing the graph's name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Sets the name of the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for the graph
  pub fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  /// Returns a guard over the nodes map. Use `.values()` to iterate node references.
  ///
  /// # Returns
  ///
  /// A mutex guard that derefs to the nodes `HashMap`.
  pub fn get_nodes(&self) -> std::sync::MutexGuard<'_, HashMap<String, Box<dyn Node>>> {
    self.nodes.lock().unwrap()
  }

  /// Returns a guard over the nodes map if the given node exists. Use `.get(name)` to get the node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to find
  ///
  /// # Returns
  ///
  /// `Some(guard)` if the node exists; the guard derefs to the nodes `HashMap`.
  pub fn find_node_by_name(
    &self,
    name: &str,
  ) -> Option<std::sync::MutexGuard<'_, HashMap<String, Box<dyn Node>>>> {
    let guard = self.nodes.lock().unwrap();
    if guard.contains_key(name) {
      Some(guard)
    } else {
      None
    }
  }

  /// Returns the number of nodes in the graph.
  ///
  /// # Returns
  ///
  /// The number of nodes in the graph.
  pub fn node_count(&self) -> usize {
    self.nodes.lock().unwrap().len()
  }

  /// Returns the number of edges in the graph.
  ///
  /// # Returns
  ///
  /// The number of edges in the graph.
  pub fn edge_count(&self) -> usize {
    self.edges.len()
  }

  /// Checks if a node with the given name exists in the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to check
  ///
  /// # Returns
  ///
  /// `true` if a node with the given name exists, `false` otherwise.
  pub fn has_node(&self, name: &str) -> bool {
    self.nodes.lock().unwrap().contains_key(name)
  }

  /// Checks if an edge exists between two nodes and ports.
  ///
  /// # Arguments
  ///
  /// * `source_node` - The name of the source node
  /// * `target_node` - The name of the target node
  ///
  /// # Returns
  ///
  /// `true` if an edge exists between the nodes, `false` otherwise.
  pub fn has_edge(&self, source_node: &str, target_node: &str) -> bool {
    self
      .edges
      .iter()
      .any(|e| e.source_node() == source_node && e.target_node() == target_node)
  }

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
  pub fn add_node(&mut self, name: String, node: Box<dyn Node>) -> Result<(), String> {
    let mut g = self.nodes.lock().unwrap();
    if g.contains_key(&name) {
      return Err(format!("Node with name '{}' already exists", name));
    }
    g.insert(name, node);
    Ok(())
  }

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
  pub fn remove_node(&mut self, name: &str) -> Result<(), String> {
    let mut g = self.nodes.lock().unwrap();
    if !g.contains_key(name) {
      return Err(format!("Node with name '{}' does not exist", name));
    }

    let has_edges = self
      .edges
      .iter()
      .any(|e| e.source_node() == name || e.target_node() == name);

    if has_edges {
      return Err(format!(
        "Cannot remove node '{}': it has connected edges",
        name
      ));
    }

    g.remove(name);
    Ok(())
  }

  /// Returns all edges in the graph.
  ///
  /// # Returns
  ///
  /// A vector of references to all edges in the graph.
  pub fn get_edges(&self) -> Vec<&Edge> {
    self.edges.iter().collect()
  }

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
  pub fn find_edge_by_nodes_and_ports(
    &self,
    source_node: &str,
    source_port: &str,
    target_node: &str,
    target_port: &str,
  ) -> Option<&Edge> {
    self.edges.iter().find(|e| {
      e.source_node() == source_node
        && e.source_port() == source_port
        && e.target_node() == target_node
        && e.target_port() == target_port
    })
  }

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
  pub fn add_edge(&mut self, edge: Edge) -> Result<(), String> {
    let g = self.nodes.lock().unwrap();
    // Validate source node exists
    if !g.contains_key(edge.source_node()) {
      return Err(format!(
        "Source node '{}' does not exist",
        edge.source_node()
      ));
    }

    // Validate target node exists
    if !g.contains_key(edge.target_node()) {
      return Err(format!(
        "Target node '{}' does not exist",
        edge.target_node()
      ));
    }

    // Validate ports exist
    let source_node = g.get(edge.source_node()).unwrap();
    if !source_node.has_output_port(edge.source_port()) {
      return Err(format!(
        "Source node '{}' does not have output port '{}'",
        edge.source_node(),
        edge.source_port()
      ));
    }

    let target_node = g.get(edge.target_node()).unwrap();
    if !target_node.has_input_port(edge.target_port()) {
      return Err(format!(
        "Target node '{}' does not have input port '{}'",
        edge.target_node(),
        edge.target_port()
      ));
    }
    drop(g);

    // Check for duplicates
    if self
      .find_edge_by_nodes_and_ports(
        edge.source_node(),
        edge.source_port(),
        edge.target_node(),
        edge.target_port(),
      )
      .is_some()
    {
      return Err("Edge already exists".to_string());
    }

    self.edges.push(edge);
    Ok(())
  }

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
  pub fn remove_edge(
    &mut self,
    source_node: &str,
    source_port: &str,
    target_node: &str,
    target_port: &str,
  ) -> Result<(), String> {
    let index = self
      .edges
      .iter()
      .position(|e| {
        e.source_node() == source_node
          && e.source_port() == source_port
          && e.target_node() == target_node
          && e.target_port() == target_port
      })
      .ok_or_else(|| "Edge not found".to_string())?;

    self.edges.remove(index);
    Ok(())
  }

  /// Executes the graph by connecting streams between nodes.
  ///
  /// This method:
  /// 1. Routes external input streams to exposed input ports
  /// 2. Performs topological sort to determine execution order
  /// 3. For each node, collects input streams from connected upstream nodes
  /// 4. Calls `node.execute(inputs)` which returns output streams
  /// 5. Routes exposed output ports to external output senders
  /// 6. Connects output streams to downstream nodes' input streams
  /// 7. Drives all streams to completion
  ///
  /// Channels are used internally for backpressure, but are never exposed to nodes.
  /// Nodes only see and work with streams. External I/O is handled through exposed ports.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was started successfully, or an error if:
  /// - Nodes have invalid port configurations
  /// - Streams cannot be created or connected
  /// - Tasks cannot be spawned
  /// - External I/O connections are invalid
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if execution cannot be started.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// use tokio::sync::mpsc;
  /// #[tokio::main]
  /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
  ///   let mut graph = Graph::new("g".to_string());
  ///   graph.add_node("map".to_string(), Box::new(VariableNode::new("map".to_string())))?;
  ///   graph.expose_input_port("map", "value", "configuration")?;
  ///   graph.expose_output_port("map", "out", "output")?;
  ///   let (config_tx, config_rx) = mpsc::channel(1);
  ///   let (output_tx, _output_rx) = mpsc::channel(10);
  ///   graph.connect_input_channel("configuration", config_rx)?;
  ///   graph.connect_output_channel("output", output_tx)?;
  ///   graph.execute().await.unwrap();
  ///   graph.wait_for_completion().await.unwrap();
  ///   Ok(())
  /// }
  /// ```
  pub async fn execute(&mut self) -> Result<(), GraphExecutionError> {
    // Create input streams from connected channels
    let mut external_inputs = HashMap::new();
    for (port_name, receiver_option) in &mut self.connected_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }

    // Use connected output channels
    let external_outputs = &self.connected_output_channels;

    // Dataflow model: take nodes out so each can run in its own task; restore in wait_for_completion
    let nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };
    let (handles, nodes_restored) = self
      .run_dataflow(nodes, Some(external_inputs), Some(external_outputs))
      .await?;
    self.execution_handles.lock().await.clear();
    self.execution_handles.lock().await.extend(handles);
    self.nodes_restored_after_run = Some(nodes_restored);
    Ok(())
  }

  /// Dataflow execution: one channel per edge, one task per node. Supports cycles.
  /// Returns (handles, nodes_restored) so callers can await and restore nodes.
  async fn run_dataflow(
    &self,
    mut nodes: HashMap<String, Box<dyn Node>>,
    external_inputs: Option<HashMap<String, crate::node::InputStream>>,
    external_outputs: Option<
      &HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
    >,
  ) -> Result<
    (
      Vec<JoinHandle<Result<(), GraphExecutionError>>>,
      NodesRestoredMap,
    ),
    GraphExecutionError,
  > {
    let edges = self.get_edges();
    type Payload = Arc<dyn Any + Send + Sync>;

    // One channel per edge: (target_node, target_port) -> receiver
    let mut input_rx: HashMap<(String, String), tokio::sync::mpsc::Receiver<Payload>> =
      HashMap::new();
    // (source_node, source_port) -> list of senders (one per edge; plus external if exposed)
    let mut output_txs: HashMap<(String, String), Vec<tokio::sync::mpsc::Sender<Payload>>> =
      HashMap::new();

    for edge in &edges {
      let (tx, rx) = tokio::sync::mpsc::channel(DATAFLOW_CHANNEL_CAPACITY);
      input_rx.insert(
        (
          edge.target_node().to_string(),
          edge.target_port().to_string(),
        ),
        rx,
      );
      output_txs
        .entry((
          edge.source_node().to_string(),
          edge.source_port().to_string(),
        ))
        .or_default()
        .push(tx);
    }

    let nodes_restored = Arc::new(Mutex::new(HashMap::new()));
    let mut all_handles = Vec::new();

    // Route external input streams into channels and give receiver to the target port
    if let Some(mut external_inputs) = external_inputs {
      for (external_port, stream) in external_inputs.drain() {
        if let Some(mapping) = self.input_port_mapping.get(&external_port) {
          let (tx, rx) = tokio::sync::mpsc::channel(DATAFLOW_CHANNEL_CAPACITY);
          input_rx.insert((mapping.node.clone(), mapping.port.clone()), rx);
          let stop_signal = Arc::clone(&self.stop_signal);
          let handle = tokio::spawn(async move {
            let mut stream = stream;
            loop {
              tokio::select! {
                _ = stop_signal.notified() => break,
                item = stream.next() => {
                  match item {
                    Some(item) => {
                      if tx.send(item).await.is_err() {
                        break;
                      }
                    }
                    None => break,
                  }
                }
              }
            }
            drop(tx);
            Ok(()) as Result<(), GraphExecutionError>
          });
          all_handles.push(handle);
        }
      }
    }

    let stop_signal = Arc::clone(&self.stop_signal);
    let output_port_mapping = self.output_port_mapping.clone();

    for (node_name, node) in nodes.drain() {
      let node_input_keys: Vec<(String, String)> = input_rx
        .keys()
        .filter(|(n, _)| n == &node_name)
        .cloned()
        .collect();
      let mut input_streams: InputStreams = HashMap::new();
      for (_, port) in node_input_keys {
        if let Some(rx) = input_rx.remove(&(node_name.clone(), port.clone())) {
          let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
          input_streams.insert(port, Box::pin(stream) as crate::node::InputStream);
        }
      }
      let output_txs_for_node: HashMap<String, Vec<tokio::sync::mpsc::Sender<Payload>>> =
        output_txs
          .iter()
          .filter(|((n, _), _)| n == &node_name)
          .map(|(k, v)| (k.1.clone(), v.clone()))
          .collect();
      let nodes_restored = Arc::clone(&nodes_restored);
      let stop_signal = Arc::clone(&stop_signal);
      let output_port_mapping = output_port_mapping.clone();
      let external_outputs = external_outputs.cloned();

      let handle = tokio::spawn(async move {
        let node_outputs = match node.execute(input_streams).await {
          Ok(o) => o,
          Err(e) => {
            let _ = nodes_restored.lock().await.insert(node_name.clone(), node);
            return Err(format!("Node '{}' execution error: {}", node_name, e).into());
          }
        };

        nodes_restored.lock().await.insert(node_name.clone(), node);

        let mut forwarder_handles = Vec::new();
        for (port_name, stream) in node_outputs {
          let mut senders = output_txs_for_node
            .get(&port_name)
            .cloned()
            .unwrap_or_default();
          let is_exposed = output_port_mapping
            .values()
            .any(|mapping| mapping.node == node_name && mapping.port == port_name);
          if is_exposed
            && let Some(ref outputs) = external_outputs
            && let Some((external_port, _)) = output_port_mapping
              .iter()
              .find(|(_, m)| m.node == node_name && m.port == port_name)
            && let Some(tx) = outputs.get(external_port)
          {
            senders.push(tx.clone());
          }

          let stop_signal = Arc::clone(&stop_signal);
          let handle = tokio::spawn(async move {
            let mut stream = stream;
            loop {
              tokio::select! {
                _ = stop_signal.notified() => break,
                item = stream.next() => {
                  match item {
                    Some(item) => {
                      for tx in &senders {
                        if tx.send(item.clone()).await.is_err() {
                          return Ok(());
                        }
                      }
                    }
                    None => break,
                  }
                }
              }
            }
            Ok(()) as Result<(), GraphExecutionError>
          });
          forwarder_handles.push(handle);
        }

        for h in forwarder_handles {
          let _ = h.await;
        }
        Ok(())
      });
      all_handles.push(handle);
    }

    Ok((all_handles, nodes_restored))
  }

  /// Starts graph execution.
  ///
  /// Sets the execution state to running. The graph will begin processing when
  /// data arrives on input ports (pull-based model).
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was started successfully.
  pub fn start(&self) {
    self.execution_state.store(1, Ordering::Release); // 1 = running
    self.pause_signal.notify_waiters(); // Resume if paused
  }

  /// Pauses graph execution.
  ///
  /// The graph will stop processing new data but maintains its current state.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was paused successfully.
  pub fn pause(&self) {
    self.execution_state.store(2, Ordering::Release); // 2 = paused
  }

  /// Resumes graph execution.
  ///
  /// Resumes processing after a pause.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was resumed successfully.
  pub fn resume(&self) {
    self.execution_state.store(1, Ordering::Release); // 1 = running
    self.pause_signal.notify_waiters();
  }

  /// Stops graph execution and clears all state.
  ///
  /// This method:
  /// 1. Signals all nodes to stop processing
  /// 2. Clears all execution handles
  /// 3. Resets execution state to None
  /// 4. All data flowing through the graph is discarded
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was stopped successfully, or an error if stopping failed.
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if execution cannot be stopped gracefully.
  pub async fn stop(&self) -> Result<(), GraphExecutionError> {
    // Notify all tasks to stop
    self.stop_signal.notify_waiters();

    // Wait for all tasks to complete
    let handles = {
      let mut handles_guard = self.execution_handles.lock().await;
      std::mem::take(&mut *handles_guard)
    };

    for handle in handles {
      let _ = handle.await;
    }

    // Reset execution state
    self.execution_state.store(0, Ordering::Release); // 0 = stopped

    // Clear execution handles
    self.execution_handles.lock().await.clear();

    Ok(())
  }

  /// Internal execution method that routes external streams to internal nodes.
  ///
  /// This is called when Graph is used as a Node in another graph.
  /// Uses the same dataflow execution as execute() so nested graphs support cycles.
  async fn execute_internal(
    &self,
    external_inputs: Option<InputStreams>,
  ) -> Result<OutputStreams, GraphExecutionError> {
    type Payload = Arc<dyn Any + Send + Sync>;

    // Take nodes out of the graph so we can run dataflow
    let nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };

    // Create a channel for each exposed output port; run_dataflow will send to these
    let mut external_output_txs: HashMap<String, tokio::sync::mpsc::Sender<Payload>> =
      HashMap::new();
    let mut output_rxs: HashMap<String, tokio::sync::mpsc::Receiver<Payload>> = HashMap::new();
    for external_port in self.output_port_mapping.keys() {
      let (tx, rx) = tokio::sync::mpsc::channel(DATAFLOW_CHANNEL_CAPACITY);
      external_output_txs.insert(external_port.clone(), tx);
      output_rxs.insert(external_port.clone(), rx);
    }

    let (handles, nodes_restored) = self
      .run_dataflow(nodes, external_inputs, Some(&external_output_txs))
      .await?;

    for handle in handles {
      handle.await??;
    }

    // Restore nodes into the graph
    {
      let mut restored = nodes_restored.lock().await;
      *self.nodes.lock().unwrap() = std::mem::take(&mut *restored);
    }

    // Build OutputStreams from the receivers we created for exposed outputs
    let mut external_outputs: OutputStreams = HashMap::new();
    for (external_port, rx) in output_rxs {
      let stream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as crate::node::OutputStream;
      external_outputs.insert(external_port, stream);
    }

    Ok(external_outputs)
  }

  /// Waits for all nodes in the graph to complete execution.
  ///
  /// This method blocks until all node tasks have finished. Use this after calling
  /// `execute()` to wait for the graph to finish processing. When using the dataflow
  /// execution model, nodes are restored into the graph after all tasks complete.
  ///
  /// # Returns
  ///
  /// `Ok(())` if all nodes completed successfully, or an error if any node failed.
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if any node execution failed or if waiting timed out.
  pub async fn wait_for_completion(&mut self) -> Result<(), GraphExecutionError> {
    let handles = {
      let mut handles_guard = self.execution_handles.lock().await;
      std::mem::take(&mut *handles_guard)
    };

    // Wait for all tasks to complete
    for handle in handles {
      handle.await??;
    }

    // Restore nodes after dataflow execution so the graph can be reused
    if let Some(restore) = self.nodes_restored_after_run.take() {
      let mut map = restore.lock().await;
      *self.nodes.lock().unwrap() = std::mem::take(&mut *map);
    }

    Ok(())
  }
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

// ============================================================================
// Node Implementation for Graph
// ============================================================================

#[async_trait]
impl Node for Graph {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &self.input_port_names
  }

  fn output_port_names(&self) -> &[String] {
    &self.output_port_names
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.input_port_names.contains(&name.to_string())
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.output_port_names.contains(&name.to_string())
  }

  fn execute(
    &self,
    inputs: InputStreams,
  ) -> Pin<Box<dyn Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
    Box::pin(async move {
      // Execute the graph as a node, routing external streams to internal nodes
      self
        .execute_internal(Some(inputs))
        .await
        .map_err(|e| format!("Graph execution error: {}", e).into())
    })
  }
}
