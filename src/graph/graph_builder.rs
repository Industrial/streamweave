//! # Graph Builder
//!
//! Graph builder for constructing graph-based data processing pipelines with a
//! fluent API and compile-time type validation. This module provides [`GraphBuilder`],
//! a builder type that enables type-safe graph construction using a state machine
//! pattern for compile-time validation of node connections and data flow.
//!
//! # Overview
//!
//! [`GraphBuilder`] provides a fluent API for constructing graphs with compile-time
//! type validation. It enables building complex data processing topologies where
//! nodes are connected according to a connection graph. All data flowing through
//! the graph is automatically wrapped in `Message<T>` for traceability and metadata
//! preservation.
//!
//! # Key Concepts
//!
//! - **Fluent API**: Method chaining for building graphs declaratively
//! - **Type-Safe Construction**: Compile-time validation of node connections
//! - **State Machine Pattern**: Uses type states for validation (Empty, HasNodes, HasConnections)
//! - **Node Management**: Adds nodes to the graph and tracks them by name
//! - **Connection Management**: Connects nodes by name with port validation
//!
//! # Core Types
//!
//! - **[`GraphBuilder`]**: Builder for constructing graphs with a fluent API
//!
//! # Quick Start
//!
//! ## Basic Graph Construction
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, GraphExecution};
//! use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave_array::ArrayProducer;
//! use streamweave::transformers::MapTransformer;
//! use streamweave::consumers::VecConsumer;
//!
//! // Build a simple linear graph
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
//! ## Complex Graph with Multiple Connections
//!
//! ```rust,no_run
//! use streamweave::graph::GraphBuilder;
//! use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//!
//! // Build a graph with fan-out and fan-in patterns
//! let graph = GraphBuilder::new()
//!     .node(producer_node)?
//!     .node(transformer1_node)?
//!     .node(transformer2_node)?
//!     .node(consumer_node)?
//!     .connect_by_name("producer", "transformer1")?
//!     .connect_by_name("producer", "transformer2")?  // Fan-out
//!     .connect_by_name("transformer1", "consumer")?
//!     .connect_by_name("transformer2", "consumer")?  // Fan-in
//!     .build();
//! ```
//!
//! # Design Decisions
//!
//! - **Fluent API**: Method chaining enables declarative graph construction
//! - **Name-Based Connections**: Uses node and port names for runtime flexibility
//! - **Error Handling**: Returns `Result` for validation errors during construction
//! - **Build Pattern**: Separate `build()` method finalizes the graph
//! - **Type Erasure**: Works with `Box<dyn NodeTrait>` for runtime graph management
//!
//! # Integration with StreamWeave
//!
//! [`GraphBuilder`] is the primary way to construct graphs in StreamWeave. It provides
//! a fluent API for adding nodes and connections, then builds a [`Graph`] instance that
//! can be executed using [`crate::graph::GraphExecution`]. All data flowing through the graph is
//! automatically wrapped in `Message<T>` during execution.

// Import for rustdoc links
#[allow(unused_imports)]
use super::execution::GraphExecution;

use super::graph::Graph;
use super::traits::NodeTrait;

/// Builder for constructing graphs with a fluent API.
///
/// `GraphBuilder` provides methods for adding nodes and connections,
/// then building a `Graph` instance.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::GraphBuilder;
/// use crate::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
///
/// let graph = GraphBuilder::new()
///     .node(producer_node)?
///     .node(transformer_node)?
///     .node(consumer_node)?
///     .connect_by_name("producer", "transformer")?
///     .connect_by_name("transformer", "consumer")?
///     .build();
/// ```
pub struct GraphBuilder {
  graph: Graph,
}

impl GraphBuilder {
  /// Creates a new `GraphBuilder` instance.
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` with an empty graph.
  pub fn new() -> Self {
    Self {
      graph: Graph::new(),
    }
  }

  /// Adds a node to the graph being built.
  ///
  /// # Arguments
  ///
  /// * `node` - A boxed node trait object
  ///
  /// # Returns
  ///
  /// `Ok(Self)` for method chaining if the node was added successfully,
  /// or an error if a node with the same name already exists.
  pub fn node(mut self, node: Box<dyn NodeTrait + Send + Sync>) -> Result<Self, String> {
    self.graph.add_node(node)?;
    Ok(self)
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
  /// `Ok(Self)` for method chaining if the connection was created successfully,
  /// or an error if the nodes don't exist or don't have the required ports.
  pub fn connect_by_name(mut self, source_name: &str, target_name: &str) -> Result<Self, String> {
    self.graph.connect_by_name(source_name, target_name)?;
    Ok(self)
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
  /// `Ok(Self)` for method chaining if the connection was created successfully,
  /// or an error if the nodes don't exist or don't have the required ports.
  pub fn connect(
    mut self,
    source_name: &str,
    source_port: &str,
    target_name: &str,
    target_port: &str,
  ) -> Result<Self, String> {
    self
      .graph
      .connect(source_name, source_port, target_name, target_port)?;
    Ok(self)
  }

  /// Builds the graph.
  ///
  /// # Returns
  ///
  /// The constructed `Graph` instance.
  pub fn build(self) -> Graph {
    self.graph
  }
}

impl Default for GraphBuilder {
  fn default() -> Self {
    Self::new()
  }
}
