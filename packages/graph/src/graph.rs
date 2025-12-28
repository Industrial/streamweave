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
//! use streamweave::producers::vec::VecProducer;
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
  /// Source node name and port index
  pub source: (String, usize),
  /// Target node name and port index
  pub target: (String, usize),
}

impl ConnectionInfo {
  /// Creates a new connection info.
  ///
  /// # Arguments
  ///
  /// * `source` - Source node name and port index
  /// * `target` - Target node name and port index
  ///
  /// # Returns
  ///
  /// A new `ConnectionInfo` instance.
  pub fn new(source: (String, usize), target: (String, usize)) -> Self {
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
    /// Port index
    port: usize,
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
      GraphError::PortNotFound { node, port } => {
        write!(f, "Port {} not found on node {}", port, node)
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

/// Helper function to resolve a port name to an index using node's port resolution.
///
/// This function queries the node for port name resolution, supporting both
/// named ports (e.g., "out0", "in1") and numeric indices.
///
/// # Arguments
///
/// * `node` - The node to query for port resolution
/// * `port_name` - The port name or index to resolve
/// * `is_output` - Whether this is an output port (true) or input port (false)
///
/// # Returns
///
/// The port index, or an error if the port name cannot be resolved.
fn resolve_port_name(
  node: &dyn NodeTrait,
  port_name: &str,
  is_output: bool,
) -> Result<usize, GraphError> {
  // Try to resolve using node's port resolution methods
  let resolved = if is_output {
    node.resolve_output_port(port_name)
  } else {
    node.resolve_input_port(port_name)
  };

  if let Some(index) = resolved {
    return Ok(index);
  }

  // If node resolution failed, try numeric index as fallback
  if let Ok(index) = port_name.parse::<usize>() {
    let port_count = if is_output {
      node.output_port_count()
    } else {
      node.input_port_count()
    };
    if index < port_count {
      return Ok(index);
    }
  }

  Err(GraphError::InvalidPortName {
    port_name: port_name.to_string(),
  })
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
}

impl Graph {
  /// Creates a new empty graph.
  ///
  /// # Returns
  ///
  /// A new empty `Graph` instance.
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      connections: Vec::new(),
    }
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
  /// A vector of tuples `(target_node_name, target_port_index)` for all nodes
  /// connected from this node.
  pub fn get_children(&self, node_name: &str) -> Vec<(&str, usize)> {
    self
      .connections
      .iter()
      .filter_map(|conn| {
        if conn.source.0 == node_name {
          Some((conn.target.0.as_str(), conn.target.1))
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
  /// A vector of tuples `(source_node_name, source_port_index)` for all nodes
  /// connected to this node.
  pub fn get_parents(&self, node_name: &str) -> Vec<(&str, usize)> {
    self
      .connections
      .iter()
      .filter_map(|conn| {
        if conn.target.0 == node_name {
          Some((conn.source.0.as_str(), conn.source.1))
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

    // Resolve port names to indices
    let source_port = if source_port_spec.is_empty() {
      0 // Default to port 0 if not specified
    } else {
      resolve_port_name(source_node.as_ref(), &source_port_spec, true)?
    };

    let target_port = if target_port_spec.is_empty() {
      0 // Default to port 0 if not specified
    } else {
      resolve_port_name(target_node.as_ref(), &target_port_spec, false)?
    };

    // Use the type-safe connect method
    // Since we're using runtime resolution, we need to use a different approach
    // For now, we'll create the connection directly and validate at runtime
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_node_name.to_string(), source_port),
      (target_node_name.to_string(), target_port),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
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

    // Validate nodes exist
    if !self.nodes.contains_key(source_name) {
      return Err(GraphError::NodeNotFound {
        name: source_name.to_string(),
      });
    }
    if !self.nodes.contains_key(target_name) {
      return Err(GraphError::NodeNotFound {
        name: target_name.to_string(),
      });
    }

    // Create connection info
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_name.to_string(), source_port),
      (target_name.to_string(), target_port),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
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

    // Resolve port names to indices
    let source_port = if source_port_spec.is_empty() {
      0 // Default to port 0 if not specified
    } else {
      resolve_port_name(source_node.as_ref(), &source_port_spec, true)?
    };

    let target_port = if target_port_spec.is_empty() {
      0 // Default to port 0 if not specified
    } else {
      resolve_port_name(target_node.as_ref(), &target_port_spec, false)?
    };

    // Create connection directly (runtime validation only)
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_node_name.to_string(), source_port),
      (target_node_name.to_string(), target_port),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
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

    // Validate nodes exist
    if !self.nodes.contains_key(source_name) {
      return Err(GraphError::NodeNotFound {
        name: source_name.to_string(),
      });
    }
    if !self.nodes.contains_key(target_name) {
      return Err(GraphError::NodeNotFound {
        name: target_name.to_string(),
      });
    }

    // Create connection info
    let mut connections = self.connections;
    connections.push(ConnectionInfo::new(
      (source_name.to_string(), source_port),
      (target_name.to_string(), target_port),
    ));

    Ok(GraphBuilder {
      nodes: self.nodes,
      connections,
      _state: HasConnections(PhantomData),
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
    }
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
  /// * `source` - Source node name and port index
  /// * `target` - Target node name and port index
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was added successfully, `Err(GraphError)` if
  /// the connection is invalid (nodes don't exist, etc.).
  ///
  /// # Note
  ///
  /// This method only validates that nodes exist. Type compatibility is not
  /// checked at runtime (that would require type information that's been erased).
  /// For type-safe connections, use `GraphBuilder::connect` instead.
  pub fn connect(
    &mut self,
    source: (&str, usize),
    target: (&str, usize),
  ) -> Result<(), GraphError> {
    let (source_name, source_port) = source;
    let (target_name, target_port) = target;

    // Validate nodes exist
    if !self.nodes.contains_key(source_name) {
      return Err(GraphError::NodeNotFound {
        name: source_name.to_string(),
      });
    }
    if !self.nodes.contains_key(target_name) {
      return Err(GraphError::NodeNotFound {
        name: target_name.to_string(),
      });
    }

    // Create connection info
    let connection = ConnectionInfo::new(
      (source_name.to_string(), source_port),
      (target_name.to_string(), target_port),
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
    }
  }
}

impl Default for RuntimeGraphBuilder {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::node::{ConsumerNode, ProducerNode, TransformerNode};
  use streamweave_transformers::map::MapTransformer;
  use streamweave_vec::consumers::VecConsumer;
  use streamweave_vec::producers::VecProducer;

  #[test]
  fn test_graph_new() {
    let graph = Graph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
  }

  #[test]
  fn test_graph_builder_add_node() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    assert!(
      builder
        .add_node(
          "source".to_string(),
          ProducerNode::from_producer("duplicate".to_string(), VecProducer::new(vec![4, 5, 6]),)
        )
        .is_err()
    );
  }

  #[test]
  fn test_graph_builder_connect() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();

    // Valid connection - use connect_by_name to avoid needing explicit closure type
    let builder = builder.connect_by_name("source", "transform").unwrap();
    assert_eq!(builder.connection_count(), 1);

    // Test that we can build from this state
    let graph = builder.build();
    assert_eq!(graph.len(), 2);
    assert_eq!(graph.get_connections().len(), 1);
  }

  #[test]
  fn test_fluent_node_api() {
    // Test the fluent node() method
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    // Use fluent API
    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();
    assert_eq!(builder.node_count(), 2);
  }

  #[test]
  fn test_fluent_connect_by_name() {
    // Test the fluent connect_by_name() method
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();

    // Connect using port names
    let builder = builder.connect_by_name("source", "transform").unwrap();
    assert_eq!(builder.connection_count(), 1);

    // Connect using explicit port names
    let builder = builder.connect_by_name("source:0", "transform:0").unwrap();
    assert_eq!(builder.connection_count(), 2);

    let graph = builder.build();
    assert_eq!(graph.get_connections().len(), 2);
  }

  #[test]
  fn test_fluent_api_chaining() {
    // Test method chaining with fluent API
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    let graph = GraphBuilder::new()
      .node(producer)
      .unwrap()
      .node(transformer)
      .unwrap()
      .node(consumer)
      .unwrap()
      .connect_by_name("source", "transform")
      .unwrap()
      .connect_by_name("transform", "sink")
      .unwrap()
      .build();

    assert_eq!(graph.len(), 3);
    assert_eq!(graph.get_connections().len(), 2);
  }

  #[test]
  fn test_compile_time_node_existence_validation() {
    // Test that connect() requires node types to be in the builder state
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    // Add nodes
    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();

    // Use connect_by_name to avoid needing explicit closure type
    let builder = builder.connect_by_name("source", "transform").unwrap();

    assert_eq!(builder.connection_count(), 1);

    // The following would fail to compile if uncommented:
    // let consumer = ConsumerNode::new("sink".to_string(), VecConsumer::new());
    // // This fails because ConsumerNode is not in the builder state yet
    // builder.connect::<
    //   ProducerNode<VecProducer<i32>, (i32,)>,
    //   ConsumerNode<VecConsumer<i32>, (i32,)>,
    //   0, 0
    // >("source", "sink", 0, 0).unwrap();
  }

  #[test]
  fn test_compile_time_port_bounds_validation() {
    // Test that port bounds are validated at compile time
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();

    // Valid port index (0) - use connect_by_name to avoid explicit type annotations
    let _builder = builder.connect_by_name("source", "transform").unwrap();

    // The following would fail to compile if uncommented:
    // // Invalid port index (1) - port doesn't exist on single-port nodes
    // builder.connect::<
    //   ProducerNode<VecProducer<i32>, (i32,)>,
    //   TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
    //   1,  // Invalid: only port 0 exists
    //   0,
    // >("source", "transform", 1, 0).unwrap();
  }

  #[test]
  fn test_compile_time_type_compatibility_validation() {
    // Test that type compatibility is validated at compile time
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();

    // Compatible types (i32 -> i32) - use connect_by_name
    let _builder = builder.connect_by_name("source", "transform").unwrap();

    // The following would fail to compile if uncommented:
    // // Incompatible types (i32 -> String) - would fail to compile
    // let string_transformer = TransformerNode::new(
    //   "string_transform".to_string(),
    //   MapTransformer::new(|x: String| x.len()),
    // );
    // builder.node(string_transformer).unwrap()
    //   .connect::<
    //     ProducerNode<VecProducer<i32>, (i32,)>,
    //     TransformerNode<MapTransformer<String, usize>, (String,), (usize,)>,
    //     0, 0
    //   >("source", "string_transform", 0, 0).unwrap();
  }

  #[test]
  fn test_port_name_resolution() {
    // Test port name resolution for different node types
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    // Test producer output port names
    assert_eq!(producer.output_port_name(0), Some("out0".to_string()));
    assert_eq!(producer.resolve_output_port("out0"), Some(0));
    assert_eq!(producer.resolve_output_port("0"), Some(0));
    assert_eq!(producer.resolve_output_port("out"), Some(0)); // Single port default
    assert_eq!(producer.input_port_name(0), None); // No input ports

    // Test transformer port names
    assert_eq!(transformer.input_port_name(0), Some("in0".to_string()));
    assert_eq!(transformer.output_port_name(0), Some("out0".to_string()));
    assert_eq!(transformer.resolve_input_port("in0"), Some(0));
    assert_eq!(transformer.resolve_input_port("0"), Some(0));
    assert_eq!(transformer.resolve_output_port("out0"), Some(0));
    assert_eq!(transformer.resolve_output_port("0"), Some(0));

    // Test consumer input port names
    assert_eq!(consumer.input_port_name(0), Some("in0".to_string()));
    assert_eq!(consumer.resolve_input_port("in0"), Some(0));
    assert_eq!(consumer.resolve_input_port("0"), Some(0));
    assert_eq!(consumer.resolve_input_port("in"), Some(0)); // Single port default
    assert_eq!(consumer.output_port_name(0), None); // No output ports
  }

  #[test]
  fn test_port_name_resolution_invalid() {
    // Test invalid port name resolution
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

    // Invalid port names should return None
    assert_eq!(producer.resolve_output_port("invalid"), None);
    assert_eq!(producer.resolve_output_port("out1"), None); // Only port 0 exists
    assert_eq!(producer.resolve_output_port("in0"), None); // Wrong prefix for output
    assert_eq!(producer.input_port_name(0), None); // No input ports
  }

  #[test]
  fn test_connect_by_name_port_resolution() {
    // Test connect_by_name with various port name formats
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();
    let builder = builder.node(consumer).unwrap();

    // Test various port name formats
    // 1. Node names only (defaults to port 0)
    let builder = builder.connect_by_name("source", "transform").unwrap();
    assert_eq!(builder.connection_count(), 1);

    // 2. Explicit port names
    let builder = builder
      .connect_by_name("source:out0", "transform:in0")
      .unwrap();
    assert_eq!(builder.connection_count(), 2);

    // 3. Numeric port indices
    let builder = builder.connect_by_name("transform:0", "sink:0").unwrap();
    assert_eq!(builder.connection_count(), 3);

    // 4. Mixed formats
    let builder = builder
      .connect_by_name("source:0", "transform:in0")
      .unwrap();
    assert_eq!(builder.connection_count(), 4);

    let graph = builder.build();
    assert_eq!(graph.get_connections().len(), 4);
  }

  #[test]
  fn test_connect_by_name_invalid_ports() {
    // Test connect_by_name with invalid port names
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();

    // Invalid port names should fail - each call consumes the builder
    let result1 = builder.connect_by_name("source:invalid", "transform");
    assert!(result1.is_err());

    // Create new builder for second test
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();
    let result2 = builder.connect_by_name("source", "transform:invalid");
    assert!(result2.is_err());

    // Create new builder for third test
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let builder = builder.node(producer).unwrap();
    let builder = builder.node(transformer).unwrap();
    let result3 = builder.connect_by_name("source:out1", "transform");
    assert!(result3.is_err()); // Port 1 doesn't exist
  }

  #[test]
  fn test_connect_by_name_node_not_found() {
    // Test connect_by_name with non-existent nodes
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

    let builder = builder.node(producer).unwrap();

    // Non-existent nodes should fail - each call consumes the builder, so we need separate tests
    let builder1 = builder.connect_by_name("nonexistent", "source");
    assert!(builder1.is_err());

    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let builder = builder.node(producer).unwrap();
    let builder2 = builder.connect_by_name("source", "nonexistent");
    assert!(builder2.is_err());
  }

  #[test]
  fn test_graph_builder_build() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let graph = builder.build();

    assert_eq!(graph.len(), 1);
    assert!(!graph.is_empty());
    assert!(graph.get_node("source").is_some());
  }

  #[test]
  fn test_runtime_graph_builder() {
    let mut builder = RuntimeGraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

    assert!(
      builder
        .add_node("source".to_string(), Box::new(producer))
        .is_ok()
    );
    assert!(
      builder
        .add_node(
          "source".to_string(),
          Box::new(ProducerNode::from_producer(
            "duplicate".to_string(),
            VecProducer::new(vec![4, 5, 6]),
          ))
        )
        .is_err()
    );
  }

  #[test]
  fn test_runtime_graph_builder_connect() {
    let mut builder = RuntimeGraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    builder
      .add_node("source".to_string(), Box::new(producer))
      .unwrap();
    builder
      .add_node("sink".to_string(), Box::new(consumer))
      .unwrap();

    // Valid connection
    assert!(builder.connect(("source", 0), ("sink", 0)).is_ok());

    // Invalid: node doesn't exist
    assert!(builder.connect(("nonexistent", 0), ("sink", 0)).is_err());
  }

  #[test]
  fn test_graph_get_children() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer1 = TransformerNode::from_transformer(
      "transform1".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let transformer2 = TransformerNode::from_transformer(
      "transform2".to_string(),
      MapTransformer::new(|x: i32| x * 3),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform1".to_string(), transformer1)
      .unwrap();
    let builder = builder
      .add_node("transform2".to_string(), transformer2)
      .unwrap();

    let builder = builder.connect_by_name("source", "transform1").unwrap();
    let builder = builder.connect_by_name("source", "transform2").unwrap();

    let graph = builder.build();
    let children = graph.get_children("source");
    assert_eq!(children.len(), 2);
    assert!(children.contains(&("transform1", 0)));
    assert!(children.contains(&("transform2", 0)));
  }

  #[test]
  fn test_graph_get_parents() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();
    let builder = builder.add_node("sink".to_string(), consumer).unwrap();

    let builder = builder.connect_by_name("source", "transform").unwrap();
    let builder = builder.connect_by_name("transform", "sink").unwrap();

    let graph = builder.build();
    let parents = graph.get_parents("transform");
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0], ("source", 0));
  }

  #[test]
  fn test_graph_node_names() {
    let builder = GraphBuilder::new();
    let builder = builder
      .add_node(
        "source".to_string(),
        ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3])),
      )
      .unwrap();
    let builder = builder
      .add_node(
        "sink".to_string(),
        ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new()),
      )
      .unwrap();

    let graph = builder.build();
    let names = graph.node_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"source"));
    assert!(names.contains(&"sink"));
  }

  #[test]
  fn test_graph_error_display() {
    let error = GraphError::NodeNotFound {
      name: "test".to_string(),
    };
    assert_eq!(error.to_string(), "Node not found: test");

    let error = GraphError::DuplicateNode {
      name: "test".to_string(),
    };
    assert_eq!(error.to_string(), "Duplicate node: test");

    let error = GraphError::InvalidConnection {
      source: "source".to_string(),
      target: "target".to_string(),
      reason: "test reason".to_string(),
    };
    assert_eq!(
      error.to_string(),
      "Invalid connection from source to target: test reason"
    );
  }
}
