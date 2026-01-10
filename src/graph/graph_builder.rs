//! # Graph Builder
//!
//! This module provides the `GraphBuilder` type for constructing
//! graph-based data processing pipelines with a fluent API.
//!
//! ## GraphBuilder
//!
//! `GraphBuilder` provides a fluent API for constructing graphs:
//!
//! ```rust,no_run
//! use crate::graph::{GraphBuilder, GraphExecution};
//! use crate::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//!
//! let graph = GraphBuilder::new()
//!     .node(producer_node)?
//!     .node(transformer_node)?
//!     .node(consumer_node)?
//!     .connect_by_name("producer", "transformer")?
//!     .connect_by_name("transformer", "consumer")?
//!     .build();
//! ```

use super::execution::ExecutionMode;
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

  /// Sets the execution mode for the graph.
  ///
  /// # Arguments
  ///
  /// * `mode` - The execution mode to set
  ///
  /// # Returns
  ///
  /// `Self` for method chaining.
  pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
    self.graph.set_execution_mode(mode);
    self
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
