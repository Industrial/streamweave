//! # Graph Traits
//!
//! This module provides traits for type-erased node access in the graph structure.
//! These traits are separated from the graph module to avoid circular dependencies.

/// Trait for type-erased node access.
///
/// This trait allows nodes to be stored in a type-erased HashMap while still
/// providing access to basic node information.
///
/// ## Port System
///
/// All ports are identified by string names, not numeric indices. Nodes must
/// explicitly specify their port names when created. This enables clearer,
/// more maintainable graph definitions.
pub trait NodeTrait: Send + Sync + std::any::Any {
  /// Returns the name of this node.
  fn name(&self) -> &str;

  /// Returns the kind of this node (Producer, Transformer, or Consumer).
  fn node_kind(&self) -> NodeKind;

  /// Returns the list of input port names for this node.
  ///
  /// # Returns
  ///
  /// A vector of input port names, in the order they are defined.
  fn input_port_names(&self) -> Vec<String>;

  /// Returns the list of output port names for this node.
  ///
  /// # Returns
  ///
  /// A vector of output port names, in the order they are defined.
  fn output_port_names(&self) -> Vec<String>;

  /// Checks if this node has an input port with the given name.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The port name to check
  ///
  /// # Returns
  ///
  /// `true` if the port exists, `false` otherwise.
  fn has_input_port(&self, port_name: &str) -> bool;

  /// Checks if this node has an output port with the given name.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The port name to check
  ///
  /// # Returns
  ///
  /// `true` if the port exists, `false` otherwise.
  fn has_output_port(&self, port_name: &str) -> bool;

  /// Spawns an execution task for this node if it supports execution.
  ///
  /// This method creates a task that will execute the node's logic (produce,
  /// transform, or consume) and communicate with other nodes via channels.
  ///
  /// # Arguments
  ///
  /// * `input_channels` - Map of (port_name, receiver) for input ports
  /// * `output_channels` - Map of (port_name, sender) for output ports
  /// * `pause_signal` - Shared signal for pausing execution
  /// * `execution_mode` - The execution mode (InProcess, Distributed, or Hybrid)
  /// * `batching_channels` - Optional map of (port_name, batching_channel) for batching
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
    _input_channels: std::collections::HashMap<String, crate::channels::TypeErasedReceiver>,
    _output_channels: std::collections::HashMap<String, crate::channels::TypeErasedSender>,
    _pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    _execution_mode: crate::execution::ExecutionMode,
    _batching_channels: Option<
      std::collections::HashMap<String, std::sync::Arc<crate::batching::BatchingChannel>>,
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
