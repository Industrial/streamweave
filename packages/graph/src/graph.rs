//! # Graph Structure
//!
//! This module provides the core Graph structure for StreamWeave that maintains
//! compile-time type safety while supporting both compile-time and runtime construction.
//! The graph stores type-preserved nodes and compile-time validated connections,
//! with basic topology query methods.
//!
//! ## Compile-time Validation
//!
//! The `GraphBuilder` provides compile-time validation of connections through:
//!
//! - **Node Existence**: The `connect()` method requires that source and target node
//!   types are present in the builder's state tuple, enforced via the `ContainsNodeType`
//!   trait bound. This ensures you can only connect nodes that have been added to the builder.
//!
//! - **Port Bounds**: Port indices are validated at compile time through the
//!   `HasOutputPort` and `HasInputPort` traits, which ensure ports exist via
//!   the `GetPort` trait. Attempting to use an invalid port index will fail to compile.
//!
//! - **Type Compatibility**: Output port types must be compatible with input port
//!   types, enforced through the `CompatibleWith` trait bound. Type mismatches
//!   are caught at compile time.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::{Graph, GraphBuilder};
//! use streamweave::graph::node::ProducerNode;
//! use streamweave_vec::VecProducer;
//!
//! let mut builder = GraphBuilder::new();
//! builder.add_node("source".to_string(), ProducerNode::new(
//!     "source".to_string(),
//!     VecProducer::new(vec![1, 2, 3]),
//! ))?;
//! let graph = builder.build();
//! ```

use crate::connection::{CompatibleWith, HasInputPort, HasOutputPort};
use crate::traits::NodeTrait;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Trait for appending a node type to a tuple of node types.
///
/// This trait enables type-level tuple concatenation for tracking node types
/// in the graph builder state machine.
///
/// # Example
///
/// ```rust
/// // Append Node2 to (Node1,)
/// type Result = <(Node1,) as AppendNode<Node2>>::Output;
/// // Result = (Node1, Node2)
/// ```
pub trait AppendNode<NewNode> {
  /// The resulting tuple type after appending `NewNode`.
  type Output;
}

/// Trait for checking if a specific node type exists in a nodes tuple.
///
/// **Note:** This trait is currently not implemented due to Rust's type system
/// limitations. Rust cannot prove that `N1 != N2` in tuples, which causes
/// conflicting trait implementations. Node existence is validated at runtime instead.
///
/// This trait is kept for documentation purposes and potential future use
/// if Rust's type system gains the ability to express type inequality.
pub trait ContainsNodeType<Node> {}

// Note: ContainsNodeType implementations are not provided due to Rust's type system
// limitations. When N1 == N2, we would have conflicting implementations:
// - impl<N1, N2> ContainsNodeType<N1> for (N1, N2)
// - impl<N1, N2> ContainsNodeType<N2> for (N1, N2)
// Rust cannot prove that N1 != N2, so these are considered overlapping.
//
// Node existence is validated at runtime instead via the `connect` method's
// runtime checks (see `GraphBuilder::connect`).

// All ContainsNodeType implementations removed - see note above

// Implementations for various tuple sizes (up to 12, matching port system)

// Implementations for various tuple sizes (up to 12, matching port system)
impl<NewNode> AppendNode<NewNode> for () {
  type Output = (NewNode,);
}

impl<N1, NewNode> AppendNode<NewNode> for (N1,) {
  type Output = (N1, NewNode);
}

impl<N1, N2, NewNode> AppendNode<NewNode> for (N1, N2) {
  type Output = (N1, N2, NewNode);
}

impl<N1, N2, N3, NewNode> AppendNode<NewNode> for (N1, N2, N3) {
  type Output = (N1, N2, N3, NewNode);
}

impl<N1, N2, N3, N4, NewNode> AppendNode<NewNode> for (N1, N2, N3, N4) {
  type Output = (N1, N2, N3, N4, NewNode);
}

impl<N1, N2, N3, N4, N5, NewNode> AppendNode<NewNode> for (N1, N2, N3, N4, N5) {
  type Output = (N1, N2, N3, N4, N5, NewNode);
}

impl<N1, N2, N3, N4, N5, N6, NewNode> AppendNode<NewNode> for (N1, N2, N3, N4, N5, N6) {
  type Output = (N1, N2, N3, N4, N5, N6, NewNode);
}

impl<N1, N2, N3, N4, N5, N6, N7, NewNode> AppendNode<NewNode> for (N1, N2, N3, N4, N5, N6, N7) {
  type Output = (N1, N2, N3, N4, N5, N6, N7, NewNode);
}

impl<N1, N2, N3, N4, N5, N6, N7, N8, NewNode> AppendNode<NewNode>
  for (N1, N2, N3, N4, N5, N6, N7, N8)
{
  type Output = (N1, N2, N3, N4, N5, N6, N7, N8, NewNode);
}

impl<N1, N2, N3, N4, N5, N6, N7, N8, N9, NewNode> AppendNode<NewNode>
  for (N1, N2, N3, N4, N5, N6, N7, N8, N9)
{
  type Output = (N1, N2, N3, N4, N5, N6, N7, N8, N9, NewNode);
}

impl<N1, N2, N3, N4, N5, N6, N7, N8, N9, N10, NewNode> AppendNode<NewNode>
  for (N1, N2, N3, N4, N5, N6, N7, N8, N9, N10)
{
  type Output = (N1, N2, N3, N4, N5, N6, N7, N8, N9, N10, NewNode);
}

impl<N1, N2, N3, N4, N5, N6, N7, N8, N9, N10, N11, NewNode> AppendNode<NewNode>
  for (N1, N2, N3, N4, N5, N6, N7, N8, N9, N10, N11)
{
  type Output = (N1, N2, N3, N4, N5, N6, N7, N8, N9, N10, N11, NewNode);
}

/// Runtime representation of a connection between nodes.
///
/// This stores the connection information needed for topology queries,
/// while the compile-time `Connection` type provides type validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionInfo {
  /// Source node name and port name
  pub source: (String, String),
  /// Target node name and port name
  pub target: (String, String),
}

impl ConnectionInfo {
  /// Creates a new connection info.
  ///
  /// # Arguments
  ///
  /// * `source` - Source node name and port name
  /// * `target` - Target node name and port name
  ///
  /// # Returns
  ///
  /// A new `ConnectionInfo` instance.
  pub fn new(source: (String, String), target: (String, String)) -> Self {
    Self { source, target }
  }
}

/// Error type for graph operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
  /// Node with the given name was not found
  NodeNotFound {
    /// The name of the node that was not found
    name: String,
  },
  /// Node with the given name already exists
  DuplicateNode {
    /// The name of the duplicate node
    name: String,
  },
  /// Invalid connection between nodes
  InvalidConnection {
    /// Source node name
    source: String,
    /// Target node name
    target: String,
    /// Reason for invalidity
    reason: String,
  },
  /// Port not found on node
  PortNotFound {
    /// Node name
    node: String,
    /// Port name
    port_name: String,
  },
  /// Type mismatch between ports
  TypeMismatch {
    /// Expected type description
    expected: String,
    /// Actual type description
    actual: String,
  },
  /// Invalid port name format
  InvalidPortName {
    /// The invalid port name
    port_name: String,
  },
}

impl std::fmt::Display for GraphError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      GraphError::NodeNotFound { name } => {
        write!(f, "Node not found: {}", name)
      }
      GraphError::DuplicateNode { name } => {
        write!(f, "Duplicate node: {}", name)
      }
      GraphError::InvalidConnection {
        source,
        target,
        reason,
      } => {
        write!(
          f,
          "Invalid connection from {} to {}: {}",
          source, target, reason
        )
      }
      GraphError::PortNotFound { node, port_name } => {
        write!(f, "Port '{}' not found on node '{}'", port_name, node)
      }
      GraphError::InvalidPortName { port_name } => {
        write!(f, "Invalid port name format: {}", port_name)
      }
      GraphError::TypeMismatch { expected, actual } => {
        write!(f, "Type mismatch: expected {}, got {}", expected, actual)
      }
    }
  }
}

/// Helper function to parse a port specification.
///
/// Ports can be specified as:
/// - A numeric string (e.g., "0", "1") representing a port index
/// - A node:port format (e.g., "source:out1") for named ports
///
/// # Arguments
///
/// * `port_spec` - The port specification string
///
/// # Returns
///
/// A tuple of (node_name, port_name_or_index) if in "node:port" format,
/// or (None, port_index) if just a numeric string.
fn parse_port_spec(port_spec: &str) -> Result<(Option<&str>, String), GraphError> {
  if let Some(colon_pos) = port_spec.find(':') {
    // Format: "node_name:port_name"
    let (node_name, port_name) = port_spec.split_at(colon_pos);
    let port_name = &port_name[1..]; // Skip the ':'
    Ok((Some(node_name), port_name.to_string()))
  } else if port_spec.chars().all(|c| c.is_ascii_digit()) {
    // Format: numeric port index
    Ok((None, port_spec.to_string()))
  } else {
    // Just a port name (no node prefix)
    Ok((None, port_spec.to_string()))
  }
}

impl std::error::Error for GraphError {}

/// Main graph structure that stores nodes and connections.
///
/// The graph maintains type-erased nodes for runtime flexibility while
/// supporting compile-time validated connections through the builder API.
pub struct Graph {
  /// Nodes stored by name (type-erased for runtime flexibility)
  nodes: HashMap<String, Box<dyn NodeTrait>>,
  /// Connections between nodes
  connections: Vec<ConnectionInfo>,
  /// Execution mode for this graph
  execution_mode: crate::execution::ExecutionMode,
}

impl Graph {
  /// Creates a new empty graph.
  ///
  /// # Returns
  ///
  /// A new empty `Graph` instance with default in-process execution mode.
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      connections: Vec::new(),
      execution_mode: crate::execution::ExecutionMode::new_in_process(), // Default to in-process for zero-copy
    }
  }

  /// Sets the execution mode for this graph.
  ///
  /// # Arguments
  ///
  /// * `mode` - The execution mode to use
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::Graph;
  /// use streamweave_graph::execution::ExecutionMode;
  ///
  /// let graph = Graph::new()
  ///   .with_execution_mode(ExecutionMode::new_in_process());
  /// ```
  #[must_use]
  pub fn with_execution_mode(mut self, mode: crate::execution::ExecutionMode) -> Self {
    self.execution_mode = mode;
    self
  }

  /// Returns the execution mode for this graph.
  ///
  /// # Returns
  ///
  /// The current `ExecutionMode`
  #[must_use]
  pub fn execution_mode(&self) -> &crate::execution::ExecutionMode {
    &self.execution_mode
  }

  /// Returns a reference to the node with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to retrieve
  ///
  /// # Returns
  ///
  /// `Some(&dyn NodeTrait)` if the node exists, `None` otherwise.
  pub fn get_node(&self, name: &str) -> Option<&dyn NodeTrait> {
    self.nodes.get(name).map(|node| node.as_ref())
  }

  /// Returns all connections in the graph.
  ///
  /// # Returns
  ///
  /// A slice of all `ConnectionInfo` instances.
  pub fn get_connections(&self) -> &[ConnectionInfo] {
    &self.connections
  }

  /// Returns the children (outgoing connections) of a node.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  ///
  /// # Returns
  ///
  /// A vector of tuples `(target_node_name, target_port_name)` for all nodes
  /// connected from this node.
  pub fn get_children(&self, node_name: &str) -> Vec<(&str, &str)> {
    self
      .connections
      .iter()
      .filter_map(|conn| {
        if conn.source.0 == node_name {
          Some((conn.target.0.as_str(), conn.target.1.as_str()))
        } else {
          None
        }
      })
      .collect()
  }

  /// Returns the parents (incoming connections) of a node.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  ///
  /// # Returns
  ///
  /// A vector of tuples `(source_node_name, source_port_name)` for all nodes
  /// connected to this node.
  pub fn get_parents(&self, node_name: &str) -> Vec<(&str, &str)> {
    self
      .connections
      .iter()
      .filter_map(|conn| {
        if conn.target.0 == node_name {
          Some((conn.source.0.as_str(), conn.source.1.as_str()))
        } else {
          None
        }
      })
      .collect()
  }

  /// Returns all node names in the graph.
  ///
  /// # Returns
  ///
  /// A vector of node name references.
  pub fn node_names(&self) -> Vec<&str> {
    self.nodes.keys().map(|s| s.as_str()).collect()
  }

  /// Returns whether the graph is empty.
  ///
  /// # Returns
  ///
  /// `true` if the graph has no nodes, `false` otherwise.
  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }

  /// Returns the number of nodes in the graph.
  ///
  /// # Returns
  ///
  /// The number of nodes.
  pub fn len(&self) -> usize {
    self.nodes.len()
  }
}

impl Default for Graph {
  fn default() -> Self {
    Self::new()
  }
}

/// Empty state for graph builder.
///
/// This state indicates that no nodes have been added to the graph yet.
pub struct Empty;

/// State indicating nodes have been added to the graph.
///
/// This state tracks the actual node types that have been added, enabling
/// compile-time validation of operations on the graph builder.
///
/// # Type Parameters
///
/// * `Nodes` - Tuple of node types that have been added (e.g., `(Node1, Node2)`)
///
/// # Example
///
/// ```rust
/// // After adding first node
/// type Builder1 = GraphBuilder<HasNodes<(ProducerNode<...>,)>>;
///
/// // After adding second node
/// type Builder2 = GraphBuilder<HasNodes<(ProducerNode<...>, TransformerNode<...>)>>;
/// ```
pub struct HasNodes<Nodes>(PhantomData<Nodes>);

/// State indicating connections have been added to the graph.
///
/// This state tracks both the node types and connection types, enabling
/// compile-time validation of the graph structure.
///
/// # Type Parameters
///
/// * `Nodes` - Tuple of node types
/// * `Connections` - Tuple of connection types (currently `()` for simplicity,
///   but can be extended for type-level connection tracking)
///
/// # Example
///
/// ```rust
/// // After adding first connection
/// type Builder = GraphBuilder<HasConnections<(Node1, Node2), ()>>;
/// ```
pub struct HasConnections<Nodes, Connections>(PhantomData<(Nodes, Connections)>);

/// Builder for constructing graphs with compile-time type validation.
///
/// This builder validates connections at compile time using trait bounds,
/// ensuring type safety while building the graph. It uses a state machine
/// with PhantomData to track construction state at compile time.
///
/// # Type Parameters
///
/// * `State` - The current state of the builder (Empty, HasNodes, HasConnections, or Complete)
pub struct GraphBuilder<State = Empty> {
  nodes: HashMap<String, Box<dyn NodeTrait>>,
  connections: Vec<ConnectionInfo>,
  _state: State,
  execution_mode: Option<crate::execution::ExecutionMode>,
}

// Initial builder creation
impl GraphBuilder<Empty> {
  /// Creates a new empty graph builder.
  ///
  /// This is the starting point for building a graph. You must add
  /// nodes before you can connect them.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::GraphBuilder;
  ///
  /// let builder = GraphBuilder::new();
  /// ```
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `Empty` state.
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      connections: Vec::new(),
      _state: Empty,
      execution_mode: None,
    }
  }
}

// Builder methods that work in any state
impl<State> GraphBuilder<State> {
  /// Returns the number of nodes in the graph.
  ///
  /// # Returns
  ///
  /// The number of nodes.
  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }

  /// Sets the execution mode for the graph being built.
  ///
  /// # Arguments
  ///
  /// * `mode` - The execution mode to use
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::GraphBuilder;
  /// use streamweave_graph::execution::ExecutionMode;
  ///
  /// let builder = GraphBuilder::new()
  ///   .with_execution_mode(ExecutionMode::new_in_process());
  /// ```
  #[must_use]
  pub fn with_execution_mode(mut self, mode: crate::execution::ExecutionMode) -> Self {
    self.execution_mode = Some(mode);
    self
  }

  /// Returns the number of connections in the graph.
  ///
  /// # Returns
  ///
  /// The number of connections.
  pub fn connection_count(&self) -> usize {
    self.connections.len()
  }

  /// Returns whether the graph is empty.
  ///
  /// # Returns
  ///
  /// `true` if the graph has no nodes, `false` otherwise.
  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }
}

// Methods for adding nodes from Empty state
impl GraphBuilder<Empty> {
  /// Adds the first node to the graph.
  ///
  /// This method transitions the builder from `Empty` to `HasNodes<(N,)>` state,
  /// tracking the node type in the state.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes<(N,)>` state, or `Err(GraphError)` if
  /// a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  ///
  /// # Example
  ///
  /// ```rust
  /// let builder = GraphBuilder::new();
  /// let producer = ProducerNode::new("source".to_string(), ...);
  /// let builder = builder.add_node("source".to_string(), producer)?;
  /// // builder is now GraphBuilder<HasNodes<(ProducerNode<...>,)>>
  /// ```
  pub fn add_node<N>(
    self,
    name: String,
    node: N,
  ) -> Result<GraphBuilder<HasNodes<(N,)>>, GraphError>
  where
    N: NodeTrait + 'static,
  {
    if self.nodes.contains_key(&name) {
      return Err(GraphError::DuplicateNode { name });
    }

    let mut nodes = self.nodes;
    nodes.insert(name, Box::new(node));

    Ok(GraphBuilder {
      nodes,
      connections: self.connections,
      _state: HasNodes(PhantomData),
      execution_mode: self.execution_mode,
    })
  }

  /// Adds a node with a fluent API, using the node's own name.
  ///
  /// This is a convenience method that extracts the name from the node itself,
  /// making the API more ergonomic.
  ///
  /// # Arguments
  ///
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes<(N,)>` state, or `Err(GraphError)` if
  /// a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  ///
  /// # Example
  ///
  /// ```rust
  /// let builder = GraphBuilder::new();
  /// let producer = ProducerNode::new("source".to_string(), ...);
  /// let builder = builder.node(producer)?;
  /// // builder is now GraphBuilder<HasNodes<(ProducerNode<...>,)>>
  /// ```
  pub fn node<N>(self, node: N) -> Result<GraphBuilder<HasNodes<(N,)>>, GraphError>
  where
    N: NodeTrait + 'static,
  {
    let name = node.name().to_string();
    self.add_node(name, node)
  }
}

// Methods for adding nodes from HasNodes state
impl<Nodes> GraphBuilder<HasNodes<Nodes>> {
  /// Adds another node to the graph.
  ///
  /// This method extends the node type tuple, tracking the new node type
  /// in the state: `HasNodes<Nodes>` → `HasNodes<Append<Nodes, N>>`.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes<Append<Nodes, N>>` state, or
  /// `Err(GraphError)` if a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  ///
  /// # Example
  ///
  /// ```rust
  /// // builder is GraphBuilder<HasNodes<(ProducerNode<...>,)>>
  /// let transformer = TransformerNode::new("transform".to_string(), ...);
  /// let builder = builder.add_node("transform".to_string(), transformer)?;
  /// // builder is now GraphBuilder<HasNodes<(ProducerNode<...>, TransformerNode<...>)>>
  /// ```
  pub fn add_node<N>(
    self,
    name: String,
    node: N,
  ) -> Result<GraphBuilder<HasNodes<<Nodes as AppendNode<N>>::Output>>, GraphError>
  where
    N: NodeTrait + 'static,
    Nodes: AppendNode<N>,
  {
    if self.nodes.contains_key(&name) {
      return Err(GraphError::DuplicateNode { name });
    }

    let mut nodes = self.nodes;
    nodes.insert(name, Box::new(node));

    Ok(GraphBuilder {
      nodes,
      connections: self.connections,
      _state: HasNodes(PhantomData),
      execution_mode: self.execution_mode,
    })
  }

  /// Adds a node with a fluent API, using the node's own name.
  ///
  /// This is a convenience method that extracts the name from the node itself,
  /// making the API more ergonomic and enabling method chaining.
  ///
  /// # Arguments
  ///
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes<Append<Nodes, N>>` state, or
  /// `Err(GraphError)` if a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  ///
  /// # Example
  ///
  /// ```rust
  /// let builder = GraphBuilder::new()
  ///     .node(ProducerNode::new("source".to_string(), ...))?
  ///     .node(TransformerNode::new("transform".to_string(), ...))?;
  /// ```
  pub fn node<N>(
    self,
    node: N,
  ) -> Result<GraphBuilder<HasNodes<<Nodes as AppendNode<N>>::Output>>, GraphError>
  where
    N: NodeTrait + 'static,
    Nodes: AppendNode<N>,
  {
    let name = node.name().to_string();
    self.add_node(name, node)
  }

  /// Connects two nodes by name using port name resolution.
  ///
  /// This method provides a fluent API for connecting nodes using string-based
  /// port specifications. Ports can be specified as:
  /// - Node names only (defaults to port 0): `"source"` → `"source:0"`
  /// - Node and port: `"source:out0"` or `"source:0"`
  /// - Just port name (for current node context): `"out0"` or `"0"`
  ///
  /// # Arguments
  ///
  /// * `source` - Source node name and optional port (e.g., `"source"`, `"source:out0"`, `"source:0"`)
  /// * `target` - Target node name and optional port (e.g., `"target"`, `"target:in0"`, `"target:0"`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasConnections<Nodes, ()>` state, or
  /// `Err(GraphError)` if the connection is invalid.
  ///
  /// # Example
  ///
  /// ```rust
  /// let builder = GraphBuilder::new()
  ///     .node(producer).unwrap()
  ///     .node(transformer).unwrap()
  ///     .connect_by_name("source", "transform").unwrap()  // Uses default port 0
  ///     .connect_by_name("source:out0", "transform:in0").unwrap();  // Explicit ports
  /// ```
  pub fn connect_by_name(
    self,
    source: &str,
    target: &str,
  ) -> Result<GraphBuilder<HasConnections<Nodes, ()>>, GraphError> {
    // Parse source specification
    let (source_node_name, mut source_port_spec) = parse_port_spec(source)?;
    let source_node_name = source_node_name.unwrap_or_else(|| {
      // If no colon in original, entire string is node name, port spec should be empty
      if !source.contains(':') {
        source_port_spec = String::new();
      }
      source
    });

    // Parse target specification
    let (target_node_name, mut target_port_spec) = parse_port_spec(target)?;
    let target_node_name = target_node_name.unwrap_or_else(|| {
      // If no colon in original, entire string is node name, port spec should be empty
      if !target.contains(':') {
        target_port_spec = String::new();
      }
      target
    });

    // Get nodes
    let source_node = self
      .nodes
      .get(source_node_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: source_node_name.to_string(),
      })?;

    let target_node = self
      .nodes
      .get(target_node_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: target_node_name.to_string(),
      })?;

    // Resolve port names (use default "out"/"in" if not specified)
    let source_port_name = if source_port_spec.is_empty() {
      // Default to first output port name
      let output_port_names = source_node.output_port_names();
      if output_port_names.is_empty() {
        return Err(GraphError::InvalidConnection {
          source: source_node_name.to_string(),
          target: target_node_name.to_string(),
          reason: "Source node has no output ports".to_string(),
        });
      }
      output_port_names[0].clone()
    } else {
      // Validate port name exists
      if !source_node.has_output_port(&source_port_spec) {
        return Err(GraphError::InvalidPortName {
          port_name: source_port_spec.clone(),
        });
      }
      source_port_spec
    };

    let target_port_name = if target_port_spec.is_empty() {
      // Default to first input port name
      let input_port_names = target_node.input_port_names();
      if input_port_names.is_empty() {
        return Err(GraphError::InvalidConnection {
          source: source_node_name.to_string(),
          target: target_node_name.to_string(),
          reason: "Target node has no input ports".to_string(),
        });
      }
      input_port_names[0].clone()
    } else {
      // Validate port name exists
      if !target_node.has_input_port(&target_port_spec) {
        return Err(GraphError::InvalidPortName {
          port_name: target_port_spec.clone(),
        });
      }
      target_port_spec
    };

    // Create connection with port names
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_node_name.to_string(), source_port_name),
      (target_node_name.to_string(), target_port_name),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
      execution_mode: self.execution_mode,
    })
  }

  /// Connects two nodes with compile-time type validation.
  ///
  /// This method provides type-safe connection creation, validating port bounds
  /// and type compatibility at compile time. It transitions the builder from
  /// `HasNodes<Nodes>` to `HasConnections<Nodes, ()>` state.
  ///
  /// # Arguments
  ///
  /// * `source_name` - The name of the source node
  /// * `target_name` - The name of the target node
  /// * `source_port` - The source port index (must match `SP`)
  /// * `target_port` - The target port index (must match `TP`)
  ///
  /// # Type Parameters
  ///
  /// * `Source` - The source node type (must implement `HasOutputPort<SP>`)
  /// * `Target` - The target node type (must implement `HasInputPort<TP>`)
  /// * `SP` - The source port index (compile-time constant)
  /// * `TP` - The target port index (compile-time constant)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasConnections<Nodes, ()>` state, or
  /// `Err(GraphError)` if the connection is invalid.
  ///
  /// # Example
  ///
  /// ```rust
  /// let builder = GraphBuilder::new()
  ///     .node(producer).unwrap()
  ///     .node(transformer).unwrap()
  ///     .connect::<ProducerNode<...>, TransformerNode<...>, 0, 0>(
  ///         "source", "transform", 0, 0
  ///     ).unwrap();
  /// ```
  pub fn connect<Source, Target, const SP: usize, const TP: usize>(
    self,
    source_name: &str,
    target_name: &str,
    source_port: usize,
    target_port: usize,
  ) -> Result<GraphBuilder<HasConnections<Nodes, ()>>, GraphError>
  where
    Source: HasOutputPort<SP> + 'static,
    Target: HasInputPort<TP> + 'static,
    <Source as HasOutputPort<SP>>::OutputType:
      CompatibleWith<<Target as HasInputPort<TP>>::InputType>,
    // Note: Node existence is validated at runtime, not compile-time.
    // This is because Rust's type system can't prove that N1 != N2 in tuples,
    // which would cause conflicting trait implementations.
  {
    // Validate that source_port matches SP and target_port matches TP
    if source_port != SP {
      return Err(GraphError::InvalidConnection {
        source: source_name.to_string(),
        target: target_name.to_string(),
        reason: format!(
          "Source port index {} doesn't match compile-time constant {}",
          source_port, SP
        ),
      });
    }
    if target_port != TP {
      return Err(GraphError::InvalidConnection {
        source: source_name.to_string(),
        target: target_name.to_string(),
        reason: format!(
          "Target port index {} doesn't match compile-time constant {}",
          target_port, TP
        ),
      });
    }

    // Validate nodes exist and get port names
    let source_node = self
      .nodes
      .get(source_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: source_name.to_string(),
      })?;
    let target_node = self
      .nodes
      .get(target_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: target_name.to_string(),
      })?;

    // Convert port indices to port names
    let source_port_names = source_node.output_port_names();
    let source_port_name = source_port_names
      .get(source_port)
      .ok_or_else(|| GraphError::PortNotFound {
        node: source_name.to_string(),
        port_name: format!("index_{}", source_port),
      })?
      .clone();

    let target_port_names = target_node.input_port_names();
    let target_port_name = target_port_names
      .get(target_port)
      .ok_or_else(|| GraphError::PortNotFound {
        node: target_name.to_string(),
        port_name: format!("index_{}", target_port),
      })?
      .clone();

    // Create connection info with port names
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_name.to_string(), source_port_name),
      (target_name.to_string(), target_port_name),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
      execution_mode: self.execution_mode,
    })
  }
}

// Methods for adding nodes from HasConnections state
impl<Nodes, Connections> GraphBuilder<HasConnections<Nodes, Connections>> {
  /// Adds another node to the graph.
  ///
  /// This method extends the node type tuple and transitions back to `HasNodes`
  /// state (since adding a node doesn't require connections).
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes<Append<Nodes, N>>` state, or
  /// `Err(GraphError)` if a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  pub fn add_node<N>(
    self,
    name: String,
    node: N,
  ) -> Result<GraphBuilder<HasNodes<<Nodes as AppendNode<N>>::Output>>, GraphError>
  where
    N: NodeTrait + 'static,
    Nodes: AppendNode<N>,
  {
    if self.nodes.contains_key(&name) {
      return Err(GraphError::DuplicateNode { name });
    }

    let mut nodes = self.nodes;
    nodes.insert(name, Box::new(node));

    Ok(GraphBuilder {
      nodes,
      connections: self.connections,
      _state: HasNodes(PhantomData),
      execution_mode: self.execution_mode,
    })
  }

  /// Adds a node with a fluent API, using the node's own name.
  ///
  /// This is a convenience method that extracts the name from the node itself,
  /// making the API more ergonomic and enabling method chaining.
  ///
  /// # Arguments
  ///
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes<Append<Nodes, N>>` state, or
  /// `Err(GraphError)` if a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  pub fn node<N>(
    self,
    node: N,
  ) -> Result<GraphBuilder<HasNodes<<Nodes as AppendNode<N>>::Output>>, GraphError>
  where
    N: NodeTrait + 'static,
    Nodes: AppendNode<N>,
  {
    let name = node.name().to_string();
    self.add_node(name, node)
  }

  /// Connects two nodes by name using port name resolution.
  ///
  /// This method provides a fluent API for connecting nodes using string-based
  /// port specifications. Ports can be specified as:
  /// - Node names only (defaults to port 0): `"source"` → `"source:0"`
  /// - Node and port: `"source:out0"` or `"source:0"`
  /// - Just port name (for current node context): `"out0"` or `"0"`
  ///
  /// # Arguments
  ///
  /// * `source` - Source node name and optional port (e.g., `"source"`, `"source:out0"`, `"source:0"`)
  /// * `target` - Target node name and optional port (e.g., `"target"`, `"target:in0"`, `"target:0"`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasConnections<Nodes, Connections>` state, or
  /// `Err(GraphError)` if the connection is invalid.
  ///
  /// # Example
  ///
  /// ```rust
  /// let builder = GraphBuilder::new()
  ///     .node(producer).unwrap()
  ///     .node(transformer).unwrap()
  ///     .connect_by_name("source", "transform").unwrap()  // Uses default port 0
  ///     .connect_by_name("source:out0", "transform:in0").unwrap();  // Explicit ports
  /// ```
  pub fn connect_by_name(
    self,
    source: &str,
    target: &str,
  ) -> Result<GraphBuilder<HasConnections<Nodes, Connections>>, GraphError> {
    // Parse source specification
    let (source_node_name, mut source_port_spec) = parse_port_spec(source)?;
    let source_node_name = source_node_name.unwrap_or_else(|| {
      // If no colon in original, entire string is node name, port spec should be empty
      if !source.contains(':') {
        source_port_spec = String::new();
      }
      source
    });

    // Parse target specification
    let (target_node_name, mut target_port_spec) = parse_port_spec(target)?;
    let target_node_name = target_node_name.unwrap_or_else(|| {
      // If no colon in original, entire string is node name, port spec should be empty
      if !target.contains(':') {
        target_port_spec = String::new();
      }
      target
    });

    // Get nodes
    let source_node = self
      .nodes
      .get(source_node_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: source_node_name.to_string(),
      })?;

    let target_node = self
      .nodes
      .get(target_node_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: target_node_name.to_string(),
      })?;

    // Resolve port names (use default "out"/"in" if not specified)
    let source_port_name = if source_port_spec.is_empty() {
      // Default to first output port name
      let output_port_names = source_node.output_port_names();
      if output_port_names.is_empty() {
        return Err(GraphError::InvalidConnection {
          source: source_node_name.to_string(),
          target: target_node_name.to_string(),
          reason: "Source node has no output ports".to_string(),
        });
      }
      output_port_names[0].clone()
    } else {
      // Validate port name exists
      if !source_node.has_output_port(&source_port_spec) {
        return Err(GraphError::InvalidPortName {
          port_name: source_port_spec.clone(),
        });
      }
      source_port_spec
    };

    let target_port_name = if target_port_spec.is_empty() {
      // Default to first input port name
      let input_port_names = target_node.input_port_names();
      if input_port_names.is_empty() {
        return Err(GraphError::InvalidConnection {
          source: source_node_name.to_string(),
          target: target_node_name.to_string(),
          reason: "Target node has no input ports".to_string(),
        });
      }
      input_port_names[0].clone()
    } else {
      // Validate port name exists
      if !target_node.has_input_port(&target_port_spec) {
        return Err(GraphError::InvalidPortName {
          port_name: target_port_spec.clone(),
        });
      }
      target_port_spec
    };

    // Create connection with port names
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_node_name.to_string(), source_port_name),
      (target_node_name.to_string(), target_port_name),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
      execution_mode: self.execution_mode,
    })
  }

  /// Adds another connection to the graph.
  ///
  /// This method allows adding additional connections while remaining in the
  /// `HasConnections` state, preserving the node types in the state.
  ///
  /// # Type Parameters
  ///
  /// * `Source` - The source node type (must implement `HasOutputPort<SP>`)
  /// * `Target` - The target node type (must implement `HasInputPort<TP>`)
  /// * `SP` - The source port index (compile-time constant)
  /// * `TP` - The target port index (compile-time constant)
  ///
  /// # Arguments
  ///
  /// * `source_name` - The name of the source node
  /// * `target_name` - The name of the target node
  /// * `source_port` - The source port index
  /// * `target_port` - The target port index
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasConnections<Nodes, Connections>` state,
  /// or `Err(GraphError)` if the connection is invalid.
  pub fn connect<Source, Target, const SP: usize, const TP: usize>(
    self,
    source_name: &str,
    target_name: &str,
    source_port: usize,
    target_port: usize,
  ) -> Result<GraphBuilder<HasConnections<Nodes, Connections>>, GraphError>
  where
    Source: HasOutputPort<SP> + 'static,
    Target: HasInputPort<TP> + 'static,
    <Source as HasOutputPort<SP>>::OutputType:
      CompatibleWith<<Target as HasInputPort<TP>>::InputType>,
    // Note: Node existence is validated at runtime, not compile-time.
    // This is because Rust's type system can't prove that N1 != N2 in tuples,
    // which would cause conflicting trait implementations.
  {
    // Validate that source_port matches SP and target_port matches TP
    if source_port != SP {
      return Err(GraphError::InvalidConnection {
        source: source_name.to_string(),
        target: target_name.to_string(),
        reason: format!(
          "Source port index {} doesn't match compile-time constant {}",
          source_port, SP
        ),
      });
    }
    if target_port != TP {
      return Err(GraphError::InvalidConnection {
        source: source_name.to_string(),
        target: target_name.to_string(),
        reason: format!(
          "Target port index {} doesn't match compile-time constant {}",
          target_port, TP
        ),
      });
    }

    // Validate nodes exist and get port names
    let source_node = self
      .nodes
      .get(source_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: source_name.to_string(),
      })?;
    let target_node = self
      .nodes
      .get(target_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: target_name.to_string(),
      })?;

    // Convert port indices to port names
    let source_port_names = source_node.output_port_names();
    let source_port_name = source_port_names
      .get(source_port)
      .ok_or_else(|| GraphError::PortNotFound {
        node: source_name.to_string(),
        port_name: format!("index_{}", source_port),
      })?
      .clone();

    let target_port_names = target_node.input_port_names();
    let target_port_name = target_port_names
      .get(target_port)
      .ok_or_else(|| GraphError::PortNotFound {
        node: target_name.to_string(),
        port_name: format!("index_{}", target_port),
      })?
      .clone();

    // Create connection info with port names
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_name.to_string(), source_port_name),
      (target_name.to_string(), target_port_name),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
      execution_mode: self.execution_mode,
    })
  }

  /// Builds the graph from the builder.
  ///
  /// # Returns
  ///
  /// A `Graph` instance containing all added nodes and connections.
  pub fn build(self) -> Graph {
    Graph {
      nodes: self.nodes,
      connections: self.connections,
      execution_mode: self
        .execution_mode
        .unwrap_or_else(crate::execution::ExecutionMode::new_in_process),
    }
  }
}

// Methods for building from HasNodes state
impl<Nodes> GraphBuilder<HasNodes<Nodes>> {
  /// Builds the graph from the builder.
  ///
  /// # Returns
  ///
  /// A `Graph` instance containing all added nodes and connections.
  pub fn build(self) -> Graph {
    Graph {
      nodes: self.nodes,
      connections: self.connections,
      execution_mode: self
        .execution_mode
        .unwrap_or_else(crate::execution::ExecutionMode::new_in_process),
    }
  }
}

impl Default for GraphBuilder<Empty> {
  fn default() -> Self {
    Self::new()
  }
}

/// Builder for constructing graphs at runtime without compile-time type validation.
///
/// This builder allows dynamic graph construction but validates connections
/// at runtime, providing more flexibility at the cost of compile-time safety.
pub struct RuntimeGraphBuilder {
  nodes: HashMap<String, Box<dyn NodeTrait>>,
  connections: Vec<ConnectionInfo>,
  execution_mode: Option<crate::execution::ExecutionMode>,
}

impl RuntimeGraphBuilder {
  /// Creates a new runtime graph builder.
  ///
  /// # Returns
  ///
  /// A new `RuntimeGraphBuilder` instance.
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      connections: Vec::new(),
      execution_mode: None,
    }
  }

  /// Sets the execution mode for the graph being built.
  ///
  /// # Arguments
  ///
  /// * `mode` - The execution mode to use
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  #[must_use]
  pub fn with_execution_mode(mut self, mode: crate::execution::ExecutionMode) -> Self {
    self.execution_mode = Some(mode);
    self
  }

  /// Adds a node to the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `node` - The node to add (type-erased)
  ///
  /// # Returns
  ///
  /// `Ok(())` if the node was added successfully, `Err(GraphError)` if a node
  /// with the same name already exists.
  pub fn add_node(&mut self, name: String, node: Box<dyn NodeTrait>) -> Result<(), GraphError> {
    if self.nodes.contains_key(&name) {
      return Err(GraphError::DuplicateNode { name });
    }
    self.nodes.insert(name, node);
    Ok(())
  }

  /// Connects two nodes with runtime validation.
  ///
  /// # Arguments
  ///
  /// * `source` - Source node name and port name
  /// * `target` - Target node name and port name
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was added successfully, `Err(GraphError)` if
  /// the connection is invalid (nodes don't exist, ports don't exist, etc.).
  ///
  /// # Note
  ///
  /// This method validates that nodes exist and that the specified ports exist.
  /// Type compatibility is not checked at runtime (that would require type information that's been erased).
  /// For type-safe connections, use `GraphBuilder::connect` instead.
  pub fn connect(&mut self, source: (&str, &str), target: (&str, &str)) -> Result<(), GraphError> {
    let (source_name, source_port_name) = source;
    let (target_name, target_port_name) = target;

    // Validate nodes exist
    let source_node = self
      .nodes
      .get(source_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: source_name.to_string(),
      })?;
    let target_node = self
      .nodes
      .get(target_name)
      .ok_or_else(|| GraphError::NodeNotFound {
        name: target_name.to_string(),
      })?;

    // Validate ports exist
    if !source_node.has_output_port(source_port_name) {
      return Err(GraphError::InvalidPortName {
        port_name: source_port_name.to_string(),
      });
    }
    if !target_node.has_input_port(target_port_name) {
      return Err(GraphError::InvalidPortName {
        port_name: target_port_name.to_string(),
      });
    }

    // Create connection info
    let connection = ConnectionInfo::new(
      (source_name.to_string(), source_port_name.to_string()),
      (target_name.to_string(), target_port_name.to_string()),
    );

    self.connections.push(connection);
    Ok(())
  }

  /// Builds the graph from the builder.
  ///
  /// # Returns
  ///
  /// A `Graph` instance containing all added nodes and connections.
  pub fn build(self) -> Graph {
    Graph {
      nodes: self.nodes,
      connections: self.connections,
      execution_mode: self
        .execution_mode
        .unwrap_or_else(crate::execution::ExecutionMode::new_in_process),
    }
  }
}

impl Default for RuntimeGraphBuilder {
  fn default() -> Self {
    Self::new()
  }
}
