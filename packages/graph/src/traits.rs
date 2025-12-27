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

  /// Spawns an execution task for this node if it supports execution.
  ///
  /// This method creates a task that will execute the node's logic (produce,
  /// transform, or consume) and communicate with other nodes via channels.
  ///
  /// # Arguments
  ///
  /// * `input_channels` - Map of (port_index, receiver) for input ports
  /// * `output_channels` - Map of (port_index, sender) for output ports
  /// * `pause_signal` - Shared signal for pausing execution
  ///
  /// # Returns
  ///
  /// `Some(JoinHandle)` if the node supports execution, `None` otherwise.
  /// The handle resolves when the node's execution completes.
  ///
  /// # Note
  ///
  /// Default implementation returns `None`. Concrete node types should
  /// override this method to provide execution capability.
  fn spawn_execution_task(
    &self,
    _input_channels: std::collections::HashMap<usize, tokio::sync::mpsc::Receiver<Vec<u8>>>,
    _output_channels: std::collections::HashMap<usize, tokio::sync::mpsc::Sender<Vec<u8>>>,
    _pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    None
  }
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
