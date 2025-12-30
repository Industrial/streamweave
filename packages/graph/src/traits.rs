//! # Graph Traits
//!
//! This module provides traits for type-erased node access in the graph structure.
//! These traits are separated from the graph module to avoid circular dependencies.

/// Trait for type-erased node access.
///
/// This trait allows nodes to be stored in a type-erased HashMap while still
/// providing access to basic node information.
pub trait NodeTrait: Send + Sync + std::any::Any {
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
  /// * `execution_mode` - The execution mode (InProcess, Distributed, or Hybrid)
  /// * `batching_channels` - Optional map of (port_index, batching_channel) for batching
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
  /// When `execution_mode` is `ExecutionMode::InProcess`, nodes should use
  /// `Arc<T>` channels instead of `Bytes` channels for zero-copy execution.
  /// Nodes receive type-erased channels and extract the appropriate type
  /// (Bytes or `Arc<T>`) based on ExecutionMode.
  /// When batching is enabled, nodes should use `batching_channels` instead of `output_channels`.
  fn spawn_execution_task(
    &self,
    _input_channels: std::collections::HashMap<usize, crate::channels::TypeErasedReceiver>,
    _output_channels: std::collections::HashMap<usize, crate::channels::TypeErasedSender>,
    _pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    _execution_mode: crate::execution::ExecutionMode,
    _batching_channels: Option<
      std::collections::HashMap<usize, std::sync::Arc<crate::batching::BatchingChannel>>,
    >,
    _arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    None
  }

  /// Try to get a reference to the StatefulNode interface if this node is stateful.
  ///
  /// # Returns
  ///
  /// `Some(&dyn StatefulNode)` if this node implements StatefulNode, `None` otherwise.
  ///
  /// # Note
  ///
  /// Default implementation returns `None`. Nodes that implement StatefulNode
  /// should override this method to return `Some(self)`.
  fn as_stateful(&self) -> Option<&dyn crate::stateful::StatefulNode> {
    None
  }

  /// Try to get a reference to the WindowedNode interface if this node is windowed.
  ///
  /// # Returns
  ///
  /// `Some(&dyn WindowedNode)` if this node implements WindowedNode, `None` otherwise.
  ///
  /// # Note
  ///
  /// Default implementation returns `None`. Nodes that implement WindowedNode
  /// should override this method to return `Some(self)`.
  fn as_windowed(&self) -> Option<&dyn crate::windowing::WindowedNode> {
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
