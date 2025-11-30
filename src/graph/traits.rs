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
}

