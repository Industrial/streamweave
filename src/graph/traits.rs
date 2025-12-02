//! # Graph Traits
//!
//! This module provides traits for type-erased node access in the graph structure.
//! These traits are separated from the graph module to avoid circular dependencies.

/// Trait for type-erased node access.
///
/// This trait allows nodes to be stored in a type-erased HashMap while still
/// providing access to basic node information.
pub trait NodeTrait: Send + Sync {
  /// Returns the name of this node.
  fn name(&self) -> &str;

  /// Returns the kind of this node (Producer, Transformer, or Consumer).
  fn node_kind(&self) -> NodeKind;

  /// Returns the number of input ports.
  fn input_port_count(&self) -> usize;

  /// Returns the number of output ports.
  fn output_port_count(&self) -> usize;

  /// Returns the name of an input port by index.
  ///
  /// # Arguments
  ///
  /// * `index` - The port index (0-based)
  ///
  /// # Returns
  ///
  /// `Some(port_name)` if the port exists, `None` otherwise.
  fn input_port_name(&self, index: usize) -> Option<String>;

  /// Returns the name of an output port by index.
  ///
  /// # Arguments
  ///
  /// * `index` - The port index (0-based)
  ///
  /// # Returns
  ///
  /// `Some(port_name)` if the port exists, `None` otherwise.
  fn output_port_name(&self, index: usize) -> Option<String>;

  /// Resolves an input port name to an index.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The port name to resolve
  ///
  /// # Returns
  ///
  /// `Some(index)` if the port name exists, `None` otherwise.
  fn resolve_input_port(&self, port_name: &str) -> Option<usize>;

  /// Resolves an output port name to an index.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The port name to resolve
  ///
  /// # Returns
  ///
  /// `Some(index)` if the port name exists, `None` otherwise.
  fn resolve_output_port(&self, port_name: &str) -> Option<usize>;
}

/// Represents the kind of a graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
  /// A producer node that generates data
  Producer,
  /// A transformer node that processes data
  Transformer,
  /// A consumer node that consumes data
  Consumer,
  /// A subgraph node that contains another graph
  Subgraph,
}
