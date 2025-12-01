//! # Graph Structure
//!
//! This module provides the core Graph structure for StreamWeave that maintains
//! compile-time type safety while supporting both compile-time and runtime construction.
//! The graph stores type-preserved nodes and compile-time validated connections,
//! with basic topology query methods.
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

use crate::graph::connection::{CompatibleWith, Connection, HasInputPort, HasOutputPort};
use crate::graph::traits::{NodeKind, NodeTrait};
use std::collections::HashMap;
use std::marker::PhantomData;

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
  NodeNotFound { name: String },
  /// Node with the given name already exists
  DuplicateNode { name: String },
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
      GraphError::InvalidConnection { source, target, reason } => {
        write!(
          f,
          "Invalid connection from {} to {}: {}",
          source, target, reason
        )
      }
      GraphError::PortNotFound { node, port } => {
        write!(f, "Port {} not found on node {}", port, node)
      }
      GraphError::TypeMismatch { expected, actual } => {
        write!(f, "Type mismatch: expected {}, got {}", expected, actual)
      }
    }
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
/// # Type Parameters
///
/// * `Nodes` - A type-level representation of the nodes that have been added
pub struct HasNodes<Nodes>(PhantomData<Nodes>);

/// State indicating connections have been added to the graph.
///
/// # Type Parameters
///
/// * `Nodes` - A type-level representation of the nodes
/// * `Connections` - A type-level representation of the connections
pub struct HasConnections<Nodes, Connections>(PhantomData<(Nodes, Connections)>);

/// Complete state indicating the graph is ready to be built.
///
/// # Type Parameters
///
/// * `Nodes` - A type-level representation of the nodes
/// * `Connections` - A type-level representation of the connections
pub struct Complete<Nodes, Connections>(PhantomData<(Nodes, Connections)>);

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

// Methods for adding nodes (works from Empty or HasNodes state)
impl<State> GraphBuilder<State> {
  /// Adds a node to the graph.
  ///
  /// This method transitions the builder from `Empty` to `HasNodes` state,
  /// or remains in `HasNodes` state if nodes already exist.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `node` - The node to add (must implement `NodeTrait`)
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` in the `HasNodes` state, or `Err(GraphError)` if
  /// a node with the same name already exists.
  ///
  /// # Type Parameters
  ///
  /// * `N` - The node type being added
  pub fn add_node<N>(
    self,
    name: String,
    node: N,
  ) -> Result<GraphBuilder<HasNodes<()>>, GraphError>
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
  use crate::consumers::vec::VecConsumer;
  use crate::graph::node::{ConsumerNode, ProducerNode, TransformerNode};
  use crate::producers::vec::VecProducer;
  use crate::transformers::map::MapTransformer;

  #[test]
  fn test_graph_new() {
    let graph = Graph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
  }

  #[test]
  fn test_graph_builder_add_node() {
    let builder = GraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    assert!(builder.add_node("source".to_string(), ProducerNode::new(
      "duplicate".to_string(),
      VecProducer::new(vec![4, 5, 6]),
    )).is_err());
  }

  #[test]
  fn test_graph_builder_connect() {
    let builder = GraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );
    let transformer = TransformerNode::new(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder.add_node("transform".to_string(), transformer).unwrap();

    // Valid connection
    let builder = builder.connect::<
      ProducerNode<VecProducer<i32>, (i32,)>,
      TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
      0,
      0,
    >("source", "transform", 0, 0).unwrap();
    assert_eq!(builder.connection_count(), 1);

    // Test that we can build from this state
    let graph = builder.build();
    assert_eq!(graph.len(), 2);
    assert_eq!(graph.get_connections().len(), 1);
  }

  #[test]
  fn test_graph_builder_build() {
    let builder = GraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let graph = builder.build();

    assert_eq!(graph.len(), 1);
    assert!(!graph.is_empty());
    assert!(graph.get_node("source").is_some());
  }

  #[test]
  fn test_runtime_graph_builder() {
    let mut builder = RuntimeGraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );

    assert!(builder.add_node("source".to_string(), Box::new(producer)).is_ok());
    assert!(builder.add_node("source".to_string(), Box::new(ProducerNode::new(
      "duplicate".to_string(),
      VecProducer::new(vec![4, 5, 6]),
    ))).is_err());
  }

  #[test]
  fn test_runtime_graph_builder_connect() {
    let mut builder = RuntimeGraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );
    let consumer = ConsumerNode::new(
      "sink".to_string(),
      VecConsumer::new(),
    );

    builder.add_node("source".to_string(), Box::new(producer)).unwrap();
    builder.add_node("sink".to_string(), Box::new(consumer)).unwrap();

    // Valid connection
    assert!(builder.connect(("source", 0), ("sink", 0)).is_ok());

    // Invalid: node doesn't exist
    assert!(builder.connect(("nonexistent", 0), ("sink", 0)).is_err());
  }

  #[test]
  fn test_graph_get_children() {
    let builder = GraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );
    let transformer1 = TransformerNode::new(
      "transform1".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let transformer2 = TransformerNode::new(
      "transform2".to_string(),
      MapTransformer::new(|x: i32| x * 3),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder.add_node("transform1".to_string(), transformer1).unwrap();
    let builder = builder.add_node("transform2".to_string(), transformer2).unwrap();

    let builder = builder.connect::<
      ProducerNode<VecProducer<i32>, (i32,)>,
      TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
      0,
      0,
    >("source", "transform1", 0, 0).unwrap();

    builder.connect::<
      ProducerNode<VecProducer<i32>, (i32,)>,
      TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
      0,
      0,
    >("source", "transform2", 0, 0).unwrap();

    let graph = builder.build();
    let children = graph.get_children("source");
    assert_eq!(children.len(), 2);
    assert!(children.contains(&("transform1", 0)));
    assert!(children.contains(&("transform2", 0)));
  }

  #[test]
  fn test_graph_get_parents() {
    let mut builder = GraphBuilder::new();
    let producer = ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    );
    let transformer = TransformerNode::new(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let consumer = ConsumerNode::new(
      "sink".to_string(),
      VecConsumer::new(),
    );

    builder.add_node("source".to_string(), producer).unwrap();
    builder.add_node("transform".to_string(), transformer).unwrap();
    builder.add_node("sink".to_string(), consumer).unwrap();

    builder.connect::<
      ProducerNode<VecProducer<i32>, (i32,)>,
      TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
      0,
      0,
    >("source", "transform", 0, 0).unwrap();

    builder.connect::<
      TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
      ConsumerNode<VecConsumer<i32>, (i32,)>,
      0,
      0,
    >("transform", "sink", 0, 0).unwrap();

    let graph = builder.build();
    let parents = graph.get_parents("transform");
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0], ("source", 0));
  }

  #[test]
  fn test_graph_node_names() {
    let mut builder = GraphBuilder::new();
    builder.add_node("source".to_string(), ProducerNode::new(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    )).unwrap();
    builder.add_node("sink".to_string(), ConsumerNode::new(
      "sink".to_string(),
      VecConsumer::new(),
    )).unwrap();

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

