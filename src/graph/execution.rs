//! # Graph Execution Engine
//!
//! This module provides the execution engine for running constructed graphs.
//! It handles concurrent node execution, stream routing via bounded channels,
//! and lifecycle management.
//!
//! The execution engine is the foundation for running graphs. It provides:
//! - Task spawning for concurrent node execution
//! - Channel-based stream routing between nodes
//! - Lifecycle management (start, stop, pause, resume)
//! - Error handling and backpressure support
//!
//! # Architecture
//!
//! The execution engine operates on a `Graph` structure that contains
//! type-erased nodes (`Box<dyn NodeTrait>`). To execute nodes, the engine
//! needs to:
//! 1. Identify node types (Producer, Transformer, Consumer) from the graph
//! 2. Spawn tasks for each node
//! 3. Create channels for routing data between nodes
//! 4. Connect streams according to the graph's connection topology
//!
//! # Execution Flow
//!
//! 1. **Initialization**: Create channels for each connection in the graph
//! 2. **Node Spawning**: Spawn tasks for each node type:
//!    - Producer nodes: Start producing and send to output channels
//!    - Transformer nodes: Receive from input channels, transform, send to output channels
//!    - Consumer nodes: Receive from input channels and consume
//! 3. **Execution**: Nodes run concurrently, with data flowing through channels
//! 4. **Shutdown**: Gracefully stop all nodes and clean up resources

use crate::graph::graph::{ConnectionInfo, Graph};
use crate::graph::traits::{NodeKind, NodeTrait};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

/// Error type for graph execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
  /// Node execution failed
  NodeExecutionFailed {
    /// Node name
    node: String,
    /// Error message
    reason: String,
  },
  /// Connection error during execution
  ConnectionError {
    /// Source node and port
    source: (String, usize),
    /// Target node and port
    target: (String, usize),
    /// Error reason
    reason: String,
  },
  /// Graph execution was cancelled
  Cancelled,
  /// Graph execution failed with an error
  ExecutionFailed(String),
  /// Invalid graph topology
  InvalidTopology(String),
}

impl std::fmt::Display for ExecutionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ExecutionError::NodeExecutionFailed { node, reason } => {
        write!(f, "Node '{}' execution failed: {}", node, reason)
      }
      ExecutionError::ConnectionError { source, target, reason } => {
        write!(
          f,
          "Connection error from {}:{} to {}:{}: {}",
          source.0, source.1, target.0, target.1, reason
        )
      }
      ExecutionError::Cancelled => write!(f, "Graph execution was cancelled"),
      ExecutionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
      ExecutionError::InvalidTopology(msg) => write!(f, "Invalid graph topology: {}", msg),
    }
  }
}

impl std::error::Error for ExecutionError {}

/// Execution state for the graph executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
  /// Graph is not running
  Stopped,
  /// Graph is running
  Running,
  /// Graph execution is paused
  Paused,
}

/// Graph execution engine that runs graphs with concurrent node execution.
///
/// The execution engine:
/// - Spawns tasks for each node
/// - Routes streams between nodes using bounded channels
/// - Manages node lifecycle (start, stop, pause, resume)
/// - Handles errors and backpressure
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::{Graph, GraphBuilder, GraphExecution};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let graph = Graph::new();
/// let mut executor = graph.executor();
///
/// // Start execution
/// executor.start().await?;
///
/// // ... graph runs concurrently ...
///
/// // Stop execution
/// executor.stop().await?;
/// # Ok(())
/// # }
/// ```
pub struct GraphExecutor {
  /// The graph to execute
  graph: Graph,
  /// Node execution handles
  node_handles: HashMap<String, JoinHandle<Result<(), ExecutionError>>>,
  /// Channel senders for routing data between nodes
  /// Key: (node_name, port_index)
  channel_senders: HashMap<(String, usize), mpsc::Sender<Vec<u8>>>,
  /// Channel receivers for routing data between nodes
  /// Key: (node_name, port_index)
  channel_receivers: HashMap<(String, usize), mpsc::Receiver<Vec<u8>>>,
  /// Execution state
  state: ExecutionState,
  /// Pause signal shared across all node tasks
  /// When true, nodes should pause execution
  pause_signal: Arc<RwLock<bool>>,
}

impl GraphExecutor {
  /// Creates a new graph executor for the given graph.
  ///
  /// # Arguments
  ///
  /// * `graph` - The graph to execute
  ///
  /// # Returns
  ///
  /// A new `GraphExecutor` instance in `Stopped` state.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  ///
  /// let graph = Graph::new();
  /// let executor = graph.executor();
  /// ```
  pub fn new(graph: Graph) -> Self {
    Self {
      graph,
      node_handles: HashMap::new(),
      channel_senders: HashMap::new(),
      channel_receivers: HashMap::new(),
      state: ExecutionState::Stopped,
      pause_signal: Arc::new(RwLock::new(false)),
    }
  }

  /// Starts graph execution.
  ///
  /// This method spawns tasks for each node and begins processing.
  /// Nodes execute concurrently, with data flowing through channels
  /// according to the graph's connection topology.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution started successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The graph is already running
  /// - Node execution fails to start
  /// - Channel creation fails
  /// - Graph topology is invalid
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// executor.start().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn start(&mut self) -> Result<(), ExecutionError> {
    if self.state == ExecutionState::Running {
      return Err(ExecutionError::ExecutionFailed(
        "Graph is already running".to_string(),
      ));
    }

    // Validate graph topology
    self.validate_topology()?;

    // Create channels for all connections
    self.create_channels()?;

    // Spawn tasks for each node
    // Note: Full implementation will be added in subsequent tasks (4.2, 4.3)
    // For now, this is a placeholder that sets the state to Running
    self.state = ExecutionState::Running;
    Ok(())
  }

  /// Stops graph execution.
  ///
  /// This method gracefully shuts down all node tasks and cleans up resources.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution stopped successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// executor.start().await?;
  /// // ... graph runs ...
  /// executor.stop().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn stop(&mut self) -> Result<(), ExecutionError> {
    if self.state == ExecutionState::Stopped {
      return Ok(());
    }

    // Cancel all node tasks
    for (node_name, handle) in self.node_handles.drain() {
      handle.abort();
      let _ = handle.await;
    }

    // Close all channels
    self.channel_senders.clear();
    self.channel_receivers.clear();

    // Clear pause signal
    *self.pause_signal.write().await = false;

    self.state = ExecutionState::Stopped;
    Ok(())
  }

  /// Returns the current execution state.
  ///
  /// # Returns
  ///
  /// The current `ExecutionState`.
  pub fn state(&self) -> ExecutionState {
    self.state
  }

  /// Returns whether the graph is currently running.
  ///
  /// # Returns
  ///
  /// `true` if the graph is running, `false` otherwise.
  pub fn is_running(&self) -> bool {
    self.state == ExecutionState::Running
  }

  /// Returns a reference to the underlying graph.
  ///
  /// # Returns
  ///
  /// A reference to the `Graph` being executed.
  pub fn graph(&self) -> &Graph {
    &self.graph
  }

  /// Pauses graph execution.
  ///
  /// This method pauses all node tasks. Nodes will stop processing new items
  /// but will not be terminated. Execution can be resumed with `resume()`.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was paused successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The graph is not running
  /// - The graph is already paused
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// executor.start().await?;
  /// executor.pause().await?;
  /// // Graph is paused, nodes are waiting
  /// executor.resume().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn pause(&mut self) -> Result<(), ExecutionError> {
    if self.state != ExecutionState::Running {
      return Err(ExecutionError::ExecutionFailed(format!(
        "Cannot pause graph in {:?} state",
        self.state
      )));
    }

    // Set pause signal
    *self.pause_signal.write().await = true;
    self.state = ExecutionState::Paused;
    Ok(())
  }

  /// Resumes graph execution from a paused state.
  ///
  /// This method resumes all node tasks that were paused. Nodes will continue
  /// processing from where they left off.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was resumed successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The graph is not paused
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// executor.start().await?;
  /// executor.pause().await?;
  /// executor.resume().await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn resume(&mut self) -> Result<(), ExecutionError> {
    if self.state != ExecutionState::Paused {
      return Err(ExecutionError::ExecutionFailed(format!(
        "Cannot resume graph in {:?} state",
        self.state
      )));
    }

    // Clear pause signal
    *self.pause_signal.write().await = false;
    self.state = ExecutionState::Running;
    Ok(())
  }

  /// Returns whether the graph is currently paused.
  ///
  /// # Returns
  ///
  /// `true` if the graph is paused, `false` otherwise.
  pub fn is_paused(&self) -> bool {
    self.state == ExecutionState::Paused
  }

  /// Returns a reference to the pause signal.
  ///
  /// This can be used by node tasks to check if they should pause.
  ///
  /// # Returns
  ///
  /// A reference to the pause signal `Arc<RwLock<bool>>`.
  pub fn pause_signal(&self) -> Arc<RwLock<bool>> {
    self.pause_signal.clone()
  }

  /// Validates the graph topology before execution.
  ///
  /// This method checks that:
  /// - All nodes referenced in connections exist
  /// - Port indices are valid for their respective nodes
  /// - The graph has at least one node
  ///
  /// # Returns
  ///
  /// `Ok(())` if the topology is valid, `Err(ExecutionError)` otherwise.
  fn validate_topology(&self) -> Result<(), ExecutionError> {
    if self.graph.is_empty() {
      return Err(ExecutionError::InvalidTopology(
        "Graph is empty".to_string(),
      ));
    }

    // Validate all connections
    for conn in self.graph.get_connections() {
      // Check source node exists
      if self.graph.get_node(&conn.source.0).is_none() {
        return Err(ExecutionError::InvalidTopology(format!(
          "Source node '{}' does not exist",
          conn.source.0
        )));
      }

      // Check target node exists
      if self.graph.get_node(&conn.target.0).is_none() {
        return Err(ExecutionError::InvalidTopology(format!(
          "Target node '{}' does not exist",
          conn.target.0
        )));
      }

      // Validate port indices
      if let Some(source_node) = self.graph.get_node(&conn.source.0) {
        if conn.source.1 >= source_node.output_port_count() {
          return Err(ExecutionError::InvalidTopology(format!(
            "Source node '{}' does not have output port {}",
            conn.source.0, conn.source.1
          )));
        }
      }

      if let Some(target_node) = self.graph.get_node(&conn.target.0) {
        if conn.target.1 >= target_node.input_port_count() {
          return Err(ExecutionError::InvalidTopology(format!(
            "Target node '{}' does not have input port {}",
            conn.target.0, conn.target.1
          )));
        }
      }
    }

    Ok(())
  }

  /// Creates channels for all connections in the graph.
  ///
  /// This method creates bounded channels for routing data between nodes.
  /// Each connection gets a channel pair (sender, receiver).
  ///
  /// # Returns
  ///
  /// `Ok(())` if channels were created successfully, `Err(ExecutionError)` otherwise.
  fn create_channels(&mut self) -> Result<(), ExecutionError> {
    const CHANNEL_BUFFER_SIZE: usize = 1024;

    for conn in self.graph.get_connections() {
      let (sender, receiver) = mpsc::channel(CHANNEL_BUFFER_SIZE);

      // Store sender for source node's output port
      self
        .channel_senders
        .insert((conn.source.0.clone(), conn.source.1), sender);

      // Store receiver for target node's input port
      self
        .channel_receivers
        .insert((conn.target.0.clone(), conn.target.1), receiver);
    }

    Ok(())
  }
}

/// Extension trait for Graph to add execution capabilities.
///
/// This trait provides a convenient way to create an executor from a graph.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::{Graph, GraphExecution};
///
/// let graph = Graph::new();
/// let executor = graph.executor();
/// ```
pub trait GraphExecution {
  /// Creates a new executor for this graph.
  ///
  /// # Returns
  ///
  /// A new `GraphExecutor` instance.
  fn executor(self) -> GraphExecutor;
}

impl GraphExecution for Graph {
  fn executor(self) -> GraphExecutor {
    GraphExecutor::new(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_graph_executor_creation() {
    let graph = Graph::new();
    let executor = graph.executor();

    assert_eq!(executor.state(), ExecutionState::Stopped);
    assert!(!executor.is_running());
  }

  #[tokio::test]
  async fn test_graph_executor_start_stop() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Start execution
    assert!(executor.start().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Running);
    assert!(executor.is_running());

    // Stop execution
    assert!(executor.stop().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Stopped);
    assert!(!executor.is_running());
  }

  #[tokio::test]
  async fn test_graph_executor_double_start() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    assert!(executor.start().await.is_ok());
    // Starting again should fail
    assert!(executor.start().await.is_err());
  }

  #[tokio::test]
  async fn test_graph_executor_empty_graph() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Starting an empty graph should fail
    assert!(executor.start().await.is_err());
  }

  #[tokio::test]
  async fn test_graph_executor_stop_when_stopped() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Stopping when already stopped should succeed
    assert!(executor.stop().await.is_ok());
  }

  #[tokio::test]
  async fn test_graph_executor_pause_resume() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Cannot pause when not running
    assert!(executor.pause().await.is_err());

    // Start execution
    assert!(executor.start().await.is_ok());

    // Pause execution
    assert!(executor.pause().await.is_ok());
    assert!(executor.is_paused());
    assert_eq!(executor.state(), ExecutionState::Paused);

    // Resume execution
    assert!(executor.resume().await.is_ok());
    assert!(!executor.is_paused());
    assert_eq!(executor.state(), ExecutionState::Running);

    // Stop execution
    assert!(executor.stop().await.is_ok());
  }

  #[tokio::test]
  async fn test_graph_executor_resume_when_not_paused() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Cannot resume when not paused
    assert!(executor.resume().await.is_err());

    // Start execution
    assert!(executor.start().await.is_ok());

    // Cannot resume when running (not paused)
    assert!(executor.resume().await.is_err());
  }

  #[tokio::test]
  async fn test_graph_executor_pause_signal() {
    let graph = Graph::new();
    let executor = graph.executor();

    // Get pause signal
    let pause_signal = executor.pause_signal();

    // Initially not paused
    assert!(!*pause_signal.read().await);

    // Set pause signal
    *pause_signal.write().await = true;
    assert!(*pause_signal.read().await);

    // Clear pause signal
    *pause_signal.write().await = false;
    assert!(!*pause_signal.read().await);
  }
}

