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
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Error type for graph execution operations.
pub type GraphExecutionError = Box<dyn std::error::Error + Send + Sync>;

/// Type alias for execution handles to reduce type complexity
type ExecutionHandleVec = Arc<Mutex<Vec<JoinHandle<Result<(), GraphExecutionError>>>>>;

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
/// use streamweave;
/// use streamweave::node::Node;
/// use streamweave::edge::Edge;
///
/// // Create a new graph
/// let mut graph = Graph::new("my_graph".to_string());
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
///
/// # Example: Nested Graph (Subgraph)
///
/// ```rust,no_run
/// use streamweave;
/// use streamweave::node::Node;
/// use streamweave::edge::Edge;
///
/// // Create a subgraph
/// let mut subgraph = Graph::new("subgraph".to_string());
/// subgraph.add_node("internal_source".to_string(), Box::new(source_node))?;
/// subgraph.add_node("internal_transform".to_string(), Box::new(transform_node))?;
/// subgraph.add_edge(Edge {
///     source_node: "internal_source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "internal_transform".to_string(),
///     target_port: "in".to_string(),
/// })?;
///
/// // Expose internal ports as external ports
/// subgraph.expose_input_port("internal_source", "in", "input")?;
/// subgraph.expose_output_port("internal_transform", "out", "output")?;
///
/// // Use subgraph as a node in parent graph
/// let mut parent = Graph::new("parent".to_string());
/// let subgraph_node: Box<dyn Node> = Box::new(subgraph);
/// parent.add_node("subgraph".to_string(), subgraph_node)?;
///
/// // Connect to subgraph's external ports
/// parent.add_edge(Edge {
///     source_node: "source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "subgraph".to_string(),
///     target_port: "input".to_string(),
/// })?;
/// ```
/// Port mapping from external port name to internal (node, port).
#[derive(Clone, Debug)]
struct PortMapping {
  /// Internal node name
  node: String,
  /// Internal port name
  port: String,
}

pub struct Graph {
  /// The name of the graph.
  name: String,
  /// Map of node names to node instances.
  nodes: HashMap<String, Box<dyn Node>>,
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
  /// use streamweave;
  ///
  /// let graph = Graph::new("my_graph".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      name,
      nodes: HashMap::new(),
      edges: Vec::new(),
      execution_handles: Arc::new(Mutex::new(Vec::new())),
      stop_signal: Arc::new(tokio::sync::Notify::new()),
      pause_signal: Arc::new(tokio::sync::Notify::new()),
      execution_state: Arc::new(AtomicU8::new(0)), // 0 = stopped
      input_port_mapping: HashMap::new(),
      output_port_mapping: HashMap::new(),
      connected_input_channels: HashMap::new(),
      connected_output_channels: HashMap::new(),
      input_port_names: vec!["configuration".to_string(), "input".to_string()],
      output_port_names: vec!["output".to_string(), "error".to_string()],
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
  /// graph.add_node("source".to_string(), Box::new(source_node))?;
  /// graph.expose_input_port("source", "in", "input")?;
  /// ```
  pub fn expose_input_port(
    &mut self,
    internal_node: &str,
    internal_port: &str,
    external_name: &str,
  ) -> Result<(), String> {
    // Validate external port name
    if external_name != "configuration" && external_name != "input" {
      return Err(format!(
        "External input port name must be 'configuration' or 'input', got '{}'",
        external_name
      ));
    }

    // Validate internal node exists
    let node = self
      .nodes
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
  /// graph.add_node("sink".to_string(), Box::new(sink_node))?;
  /// graph.expose_output_port("sink", "out", "output")?;
  /// ```
  pub fn expose_output_port(
    &mut self,
    internal_node: &str,
    internal_port: &str,
    external_name: &str,
  ) -> Result<(), String> {
    // Validate external port name
    if external_name != "output" && external_name != "error" {
      return Err(format!(
        "External output port name must be 'output' or 'error', got '{}'",
        external_name
      ));
    }

    // Validate internal node exists
    let node = self
      .nodes
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
  /// ```rust
  /// use tokio::sync::mpsc;
  ///
  /// let (tx, rx) = mpsc::channel(10);
  /// graph.connect_input_channel("configuration", rx)?;
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
  /// ```rust
  /// use tokio::sync::mpsc;
  ///
  /// let (tx, rx) = mpsc::channel(10);
  /// graph.connect_output_channel("output", tx)?;
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

  /// Returns all nodes in the graph.
  ///
  /// # Returns
  ///
  /// A vector of references to boxed nodes in the graph.
  pub fn get_nodes(&self) -> Vec<&dyn Node> {
    self.nodes.values().map(|node| node.as_ref()).collect()
  }

  /// Gets a node by name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to find
  ///
  /// # Returns
  ///
  /// `Some(&dyn Node)` if a node with the given name exists, `None` otherwise.
  pub fn find_node_by_name(&self, name: &str) -> Option<&dyn Node> {
    self.nodes.get(name).map(|node| node.as_ref())
  }

  /// Returns the number of nodes in the graph.
  ///
  /// # Returns
  ///
  /// The number of nodes in the graph.
  pub fn node_count(&self) -> usize {
    self.nodes.len()
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
    self.nodes.contains_key(name)
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
    if self.nodes.contains_key(&name) {
      return Err(format!("Node with name '{}' already exists", name));
    }
    self.nodes.insert(name, node);
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
    if !self.nodes.contains_key(name) {
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

    self.nodes.remove(name);
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
    // Validate source node exists
    if !self.nodes.contains_key(edge.source_node()) {
      return Err(format!(
        "Source node '{}' does not exist",
        edge.source_node()
      ));
    }

    // Validate target node exists
    if !self.nodes.contains_key(edge.target_node()) {
      return Err(format!(
        "Target node '{}' does not exist",
        edge.target_node()
      ));
    }

    // Validate ports exist
    let source_node = self.nodes.get(edge.source_node()).unwrap();
    if !source_node.has_output_port(edge.source_port()) {
      return Err(format!(
        "Source node '{}' does not have output port '{}'",
        edge.source_node(),
        edge.source_port()
      ));
    }

    let target_node = self.nodes.get(edge.target_node()).unwrap();
    if !target_node.has_input_port(edge.target_port()) {
      return Err(format!(
        "Target node '{}' does not have input port '{}'",
        edge.target_node(),
        edge.target_port()
      ));
    }

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
  /// use tokio::sync::mpsc;
  /// use tokio_stream::wrappers::ReceiverStream;
  ///
  /// // Build graph structure
  /// graph.add_node("map".to_string(), Box::new(map_node))?;
  /// graph.expose_input_port("map", "configuration", "configuration")?;
  /// graph.expose_output_port("map", "out", "output")?;
  ///
  /// // Connect external I/O
  /// let (config_tx, config_rx) = mpsc::channel(1);
  /// let (output_tx, output_rx) = mpsc::channel(10);
  /// graph.connect_external_input("configuration", Box::pin(ReceiverStream::new(config_rx)))?;
  /// graph.connect_external_output("output", output_tx)?;
  ///
  /// // Start execution
  /// graph.execute().await?;
  ///
  /// // Wait for completion
  /// graph.wait_for_completion().await?;
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

    self
      .execute_with_external_io(Some(external_inputs), Some(external_outputs))
      .await
  }

  /// Execute the graph with optional external I/O connections.
  ///
  /// # Arguments
  ///
  /// * `external_inputs` - Optional map of external port names to input streams (consumed)
  /// * `external_outputs` - Optional map of external port names to output senders
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was started successfully.
  pub async fn execute_with_external_io(
    &self,
    external_inputs: Option<HashMap<String, crate::node::InputStream>>,
    external_outputs: Option<
      &HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
    >,
  ) -> Result<(), GraphExecutionError> {
    // Clear any previous execution handles
    self.execution_handles.lock().await.clear();

    // Get all nodes and edges
    let nodes: Vec<&dyn Node> = self.get_nodes();
    let edges: Vec<&Edge> = self.get_edges();

    // Perform topological sort to determine execution order
    let execution_order = topological_sort(&nodes, &edges)?;

    // Build a map of output streams for each node and port
    // Key: (node_name, port_name) -> OutputStream
    let mut output_streams: HashMap<(String, String), crate::node::OutputStream> = HashMap::new();

    // Route external input streams to internal nodes
    if let Some(mut external_inputs) = external_inputs {
      for (external_port, external_stream) in external_inputs.drain() {
        if let Some(mapping) = self.input_port_mapping.get(&external_port) {
          output_streams.insert(
            (mapping.node.clone(), mapping.port.clone()),
            external_stream,
          );
        }
      }
    }

    // For each node in execution order, collect inputs and execute
    for node_name in execution_order {
      let node = self
        .nodes
        .get(&node_name)
        .ok_or_else(|| format!("Node '{}' not found", node_name))?;

      // Collect input streams from upstream nodes
      let mut input_streams: InputStreams = HashMap::new();

      // Find all edges that target this node
      for edge in &edges {
        if edge.target_node() == node_name {
          let source_node_name = edge.source_node();
          let source_port = edge.source_port();
          let target_port = edge.target_port();

          // Get the output stream from the source node
          let stream_key = (source_node_name.to_string(), source_port.to_string());
          if let Some(source_stream) = output_streams.remove(&stream_key) {
            // Move the stream to this node's input
            input_streams.insert(target_port.to_string(), source_stream);
          } else {
            return Err(
              format!(
                "Missing output stream from node '{}' port '{}'",
                source_node_name, source_port
              )
              .into(),
            );
          }
        }
      }

      // Also check for directly routed external inputs to this node
      let node_stream_keys: Vec<_> = output_streams
        .keys()
        .filter(|(n, _)| n == &node_name)
        .cloned()
        .collect();

      for stream_key in node_stream_keys {
        if let Some(stream) = output_streams.remove(&stream_key) {
          let (_, port) = stream_key;
          input_streams.insert(port, stream);
        }
      }

      // Execute the node
      let node_outputs = node.execute(input_streams).await?;

      // Store the output streams for downstream nodes and route exposed outputs
      for (port_name, stream) in node_outputs {
        let stream_key = (node_name.clone(), port_name.clone());

        // Check if this is an exposed output port
        let is_exposed_output = self
          .output_port_mapping
          .values()
          .any(|mapping| mapping.node == node_name && mapping.port == port_name);

        if is_exposed_output {
          // Find the external port name for this internal port
          if let Some((external_port, _)) = self
            .output_port_mapping
            .iter()
            .find(|(_, mapping)| mapping.node == node_name && mapping.port == port_name)
          {
            if let Some(external_sender) =
              external_outputs.and_then(|outputs| outputs.get(external_port))
            {
              // Route the stream to the external sender
              let sender = external_sender.clone();
              let stream_for_task = stream;
              let stop_signal = Arc::clone(&self.stop_signal);

              let handle = tokio::spawn(async move {
                use tokio_stream::StreamExt;
                let mut stream = stream_for_task;
                loop {
                  tokio::select! {
                    _ = stop_signal.notified() => {
                      return Ok(());
                    }
                    result = stream.next() => {
                      match result {
                        Some(item) => {
                          if sender.send(item).await.is_err() {
                            // External receiver was dropped, stop sending
                            return Ok(());
                          }
                        }
                        None => break, // Stream ended
                      }
                    }
                  }
                }
                Ok(())
              });
              self.execution_handles.lock().await.push(handle);
            } else {
              // No external sender connected, just store the stream normally
              output_streams.insert(stream_key, stream);
            }
          } else {
            // Should not happen, but store normally if mapping not found
            output_streams.insert(stream_key, stream);
          }
        } else {
          // Not an exposed output, store normally for internal routing
          output_streams.insert(stream_key, stream);
        }
      }

      // For sink nodes (no outputs), we still need to drive their execution
      // The node's execute() method should handle consuming the input streams
    }

    // Spawn tasks to drive all remaining output streams to completion
    // This ensures streams are consumed even if they're not connected to anything
    let handles = Arc::clone(&self.execution_handles);

    for (_stream_key, mut stream) in output_streams {
      let stop_signal = Arc::clone(&self.stop_signal);
      let handle = tokio::spawn(async move {
        use tokio_stream::StreamExt;
        loop {
          tokio::select! {
            _ = stop_signal.notified() => {
              return Ok(());
            }
            result = stream.next() => {
              if result.is_none() {
                break;
              }
            }
          }
        }
        Ok(())
      });
      handles.lock().await.push(handle);
    }

    Ok(())
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
  /// It routes external input streams to internal boundary nodes and collects
  /// outputs from internal boundary nodes to expose as external outputs.
  async fn execute_internal(
    &self,
    external_inputs: Option<InputStreams>,
  ) -> Result<OutputStreams, GraphExecutionError> {
    // Clear any previous execution handles
    self.execution_handles.lock().await.clear();

    // Get all nodes and edges
    let nodes: Vec<&dyn Node> = self.get_nodes();
    let edges: Vec<&Edge> = self.get_edges();

    // Perform topological sort to determine execution order
    let execution_order = topological_sort(&nodes, &edges)?;

    // Build a map of output streams for each node and port
    // Key: (node_name, port_name) -> OutputStream
    let mut output_streams: HashMap<(String, String), crate::node::OutputStream> = HashMap::new();

    // Route external input streams to internal nodes based on port mapping
    println!(
      "DEBUG: About to check external_inputs: is_some={}",
      external_inputs.is_some()
    );
    if let Some(mut external_inputs) = external_inputs {
      println!(
        "DEBUG: execute_with_external_io - routing {} external inputs",
        external_inputs.len()
      );
      for (external_port, external_stream) in external_inputs.drain() {
        println!(
          "DEBUG: execute_with_external_io - processing external port '{}'",
          external_port
        );
        if let Some(mapping) = self.input_port_mapping.get(&external_port) {
          println!(
            "DEBUG: execute_with_external_io - mapped '{}' -> '{}'.'{}'",
            external_port, mapping.node, mapping.port
          );
          // Route this external stream to the internal node's port
          let stream_key = (mapping.node.clone(), mapping.port.clone());
          println!(
            "DEBUG: execute_with_external_io - inserting stream with key {:?}",
            stream_key
          );
          output_streams.insert(stream_key, external_stream);
          println!(
            "DEBUG: execute_with_external_io - output_streams now has {} entries",
            output_streams.len()
          );
        } else {
          println!(
            "DEBUG: execute_with_external_io - no mapping for external port '{}'",
            external_port
          );
        }
      }
    } else {
      println!("DEBUG: execute_with_external_io - no external inputs provided");
    }

    // For each node in execution order, collect inputs and execute
    for node_name in execution_order {
      let node = self
        .nodes
        .get(&node_name)
        .ok_or_else(|| format!("Node '{}' not found", node_name))?;

      // Collect input streams from upstream nodes
      let mut input_streams: InputStreams = HashMap::new();

      // Find all edges that target this node
      for edge in &edges {
        if edge.target_node() == node_name {
          let source_node_name = edge.source_node();
          let source_port = edge.source_port();
          let target_port = edge.target_port();

          // Get the output stream from the source node
          let stream_key = (source_node_name.to_string(), source_port.to_string());
          if let Some(source_stream) = output_streams.remove(&stream_key) {
            // Move the stream to this node's input
            input_streams.insert(target_port.to_string(), source_stream);
          } else {
            return Err(
              format!(
                "Missing output stream from node '{}' port '{}'",
                source_node_name, source_port
              )
              .into(),
            );
          }
        }
      }

      // Also check for directly routed external inputs to this node
      let node_stream_keys: Vec<_> = output_streams
        .keys()
        .filter(|(n, _)| n == &node_name)
        .cloned()
        .collect();

      for stream_key in node_stream_keys {
        if let Some(stream) = output_streams.remove(&stream_key) {
          let (_, port) = stream_key;
          input_streams.insert(port, stream);
        }
      }

      // Execute the node
      let node_outputs = node.execute(input_streams).await?;

      // Store the output streams for downstream nodes
      for (port_name, stream) in node_outputs {
        output_streams.insert((node_name.clone(), port_name.clone()), stream);
      }
    }

    // Collect external output streams from internal boundary nodes
    let mut external_outputs: OutputStreams = HashMap::new();

    for (external_port, mapping) in &self.output_port_mapping {
      let stream_key = (mapping.node.clone(), mapping.port.clone());
      if let Some(stream) = output_streams.remove(&stream_key) {
        external_outputs.insert(external_port.clone(), stream);
      } else {
        // If the internal port doesn't have a stream, create an empty one
        // This handles the case where the internal node hasn't produced output yet
        use tokio::sync::mpsc;
        use tokio_stream::wrappers::ReceiverStream;
        let (_tx, rx) = mpsc::channel(10);
        external_outputs.insert(
          external_port.clone(),
          Box::pin(ReceiverStream::new(rx)) as crate::node::OutputStream,
        );
      }
    }

    // Spawn tasks to drive all remaining output streams to completion
    // This ensures streams are consumed even if they're not connected to anything
    let handles = Arc::clone(&self.execution_handles);
    let stop_signal = Arc::clone(&self.stop_signal);
    let pause_signal = Arc::clone(&self.pause_signal);
    let execution_state = Arc::clone(&self.execution_state);

    for (_stream_key, mut stream) in output_streams {
      let stop_signal_clone = Arc::clone(&stop_signal);
      let pause_signal_clone = Arc::clone(&pause_signal);
      let execution_state_clone = Arc::clone(&execution_state);
      let handle = tokio::spawn(async move {
        use tokio_stream::StreamExt;
        loop {
          tokio::select! {
            _ = stop_signal_clone.notified() => {
              return Ok(());
            }
            _ = pause_signal_clone.notified() => {
              // Wait for resume
              loop {
                let state = execution_state_clone.load(Ordering::Acquire);
                if state == 1 {
                  break; // Resumed (running)
                }
                if state == 0 {
                  return Ok(()); // Stopped
                }
                // state == 2 (paused), wait a bit and check again
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
              }
            }
            result = stream.next() => {
              if result.is_none() {
                break;
              }
            }
          }
        }
        Ok(())
      });
      handles.lock().await.push(handle);
    }

    Ok(external_outputs)
  }

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
  pub async fn wait_for_completion(&self) -> Result<(), GraphExecutionError> {
    let handles = {
      let mut handles_guard = self.execution_handles.lock().await;
      std::mem::take(&mut *handles_guard)
    };

    // Wait for all tasks to complete
    for handle in handles {
      handle.await??;
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
    name == "configuration" || name == "input"
  }

  fn has_output_port(&self, name: &str) -> bool {
    name == "output" || name == "error"
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
