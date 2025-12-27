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

use crate::graph::Graph;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio::time::timeout;

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
      ExecutionError::ConnectionError {
        source,
        target,
        reason,
      } => {
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
  /// Errors collected during execution
  execution_errors: Vec<ExecutionError>,
  /// Shutdown timeout duration
  shutdown_timeout: Duration,
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
      execution_errors: Vec::new(),
      shutdown_timeout: Duration::from_secs(30), // Default 30 second timeout
    }
  }

  /// Creates a new executor with a custom shutdown timeout.
  ///
  /// # Arguments
  ///
  /// * `graph` - The graph to execute
  /// * `shutdown_timeout` - Maximum time to wait for graceful shutdown
  ///
  /// # Returns
  ///
  /// A new `GraphExecutor` instance with the specified shutdown timeout.
  pub fn with_shutdown_timeout(graph: Graph, shutdown_timeout: Duration) -> Self {
    Self {
      graph,
      node_handles: HashMap::new(),
      channel_senders: HashMap::new(),
      channel_receivers: HashMap::new(),
      state: ExecutionState::Stopped,
      pause_signal: Arc::new(RwLock::new(false)),
      execution_errors: Vec::new(),
      shutdown_timeout,
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
    for node_name in self.graph.node_names() {
      if let Some(node) = self.graph.get_node(node_name) {
        // Collect input channels for this node
        let mut input_channels = HashMap::new();
        let parents = self.graph.get_parents(node_name);
        for (parent_name, parent_port) in parents {
          // Find which input port this connection targets
          for conn in self.graph.get_connections() {
            if conn.source.0 == parent_name
              && conn.source.1 == parent_port
              && conn.target.0 == node_name
            {
              let key = (parent_name.to_string(), parent_port);
              if let Some(receiver) = self.channel_receivers.remove(&key) {
                input_channels.insert(conn.target.1, receiver);
              }
              break;
            }
          }
        }

        // Collect output channels for this node
        let mut output_channels = HashMap::new();
        let children = self.graph.get_children(node_name);
        for (child_name, child_port) in children {
          // Find which output port this connection comes from
          for conn in self.graph.get_connections() {
            if conn.source.0 == node_name
              && conn.target.0 == child_name
              && conn.target.1 == child_port
            {
              let key = (node_name.to_string(), conn.source.1);
              if let Some(sender) = self.channel_senders.get(&key).cloned() {
                output_channels.insert(conn.source.1, sender);
              }
              break;
            }
          }
        }

        // Spawn execution task using the NodeTrait method
        if let Some(handle) =
          node.spawn_execution_task(input_channels, output_channels, self.pause_signal.clone())
        {
          self.node_handles.insert(node_name.to_string(), handle);
        } else {
          return Err(ExecutionError::NodeExecutionFailed {
            node: node_name.to_string(),
            reason: "Node does not support execution".to_string(),
          });
        }
      }
    }

    self.state = ExecutionState::Running;
    Ok(())
  }

  /// Stops graph execution with graceful shutdown.
  ///
  /// This method gracefully shuts down all node tasks and cleans up resources.
  /// It attempts to wait for tasks to complete naturally, but will force
  /// termination if they exceed the shutdown timeout.
  ///
  /// # Graceful Shutdown Process
  ///
  /// 1. Signal all nodes to stop (via pause signal or cancellation)
  /// 2. Wait for tasks to complete (up to shutdown_timeout)
  /// 3. Force abort any remaining tasks
  /// 4. Collect and report any errors from node tasks
  /// 5. Clean up all channels and resources
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution stopped successfully, `Err(ExecutionError)` if
  /// shutdown failed or errors occurred during execution.
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

    // Signal shutdown by setting pause (nodes should check this and exit)
    *self.pause_signal.write().await = true;

    // Close all channel senders to signal end of stream
    self.channel_senders.clear();

    // Wait for tasks to complete gracefully (with timeout)
    let handles: Vec<(String, JoinHandle<Result<(), ExecutionError>>)> =
      self.node_handles.drain().collect();

    let shutdown_result = timeout(self.shutdown_timeout, async {
      let mut errors = Vec::new();
      for (node_name, handle) in handles {
        match handle.await {
          Ok(Ok(())) => {
            // Task completed successfully
          }
          Ok(Err(e)) => {
            // Task returned an error
            errors.push(ExecutionError::NodeExecutionFailed {
              node: node_name.clone(),
              reason: e.to_string(),
            });
          }
          Err(e) => {
            // Task was cancelled or panicked
            if e.is_cancelled() {
              // Expected during shutdown
            } else if e.is_panic() {
              errors.push(ExecutionError::NodeExecutionFailed {
                node: node_name.clone(),
                reason: format!("Task panicked: {:?}", e),
              });
            }
          }
        }
      }
      errors
    })
    .await;

    // Handle timeout or collect errors
    match shutdown_result {
      Ok(errors) => {
        self.execution_errors.extend(errors);
      }
      Err(_) => {
        // Timeout occurred - force abort remaining tasks
        // (All tasks should already be collected, but handle edge cases)
        return Err(ExecutionError::ExecutionFailed(format!(
          "Shutdown timeout exceeded ({}s). Some tasks may not have completed gracefully.",
          self.shutdown_timeout.as_secs()
        )));
      }
    }

    // Clean up remaining resources
    self.channel_receivers.clear();
    *self.pause_signal.write().await = false;

    // Check if there were any errors during execution
    if !self.execution_errors.is_empty() {
      let error_summary = self
        .execution_errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ");
      return Err(ExecutionError::ExecutionFailed(format!(
        "Errors during execution: {}",
        error_summary
      )));
    }

    self.state = ExecutionState::Stopped;
    Ok(())
  }

  /// Stops graph execution immediately without graceful shutdown.
  ///
  /// This method immediately aborts all node tasks without waiting for
  /// them to complete. Use this only when graceful shutdown is not possible
  /// or when you need immediate termination.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was stopped, `Err(ExecutionError)` otherwise.
  pub async fn stop_immediate(&mut self) -> Result<(), ExecutionError> {
    if self.state == ExecutionState::Stopped {
      return Ok(());
    }

    // Immediately abort all tasks
    for (_node_name, handle) in self.node_handles.drain() {
      handle.abort();
      let _ = handle.await;
    }

    // Clean up resources
    self.channel_senders.clear();
    self.channel_receivers.clear();
    *self.pause_signal.write().await = false;

    self.state = ExecutionState::Stopped;
    Ok(())
  }

  /// Returns all errors collected during execution.
  ///
  /// # Returns
  ///
  /// A slice of all `ExecutionError` instances that occurred during execution.
  pub fn errors(&self) -> &[ExecutionError] {
    &self.execution_errors
  }

  /// Clears all collected errors.
  ///
  /// This is useful when you want to reset error state, for example
  /// after handling errors or before restarting execution.
  pub fn clear_errors(&mut self) {
    self.execution_errors.clear();
  }

  /// Returns the shutdown timeout duration.
  ///
  /// # Returns
  ///
  /// The current shutdown timeout.
  pub fn shutdown_timeout(&self) -> Duration {
    self.shutdown_timeout
  }

  /// Sets the shutdown timeout duration.
  ///
  /// # Arguments
  ///
  /// * `timeout` - The new shutdown timeout duration
  pub fn set_shutdown_timeout(&mut self, timeout: Duration) {
    self.shutdown_timeout = timeout;
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
      if let Some(source_node) = self.graph.get_node(&conn.source.0)
        && conn.source.1 >= source_node.output_port_count()
      {
        return Err(ExecutionError::InvalidTopology(format!(
          "Source node '{}' does not have output port {}",
          conn.source.0, conn.source.1
        )));
      }

      if let Some(target_node) = self.graph.get_node(&conn.target.0)
        && conn.target.1 >= target_node.input_port_count()
      {
        return Err(ExecutionError::InvalidTopology(format!(
          "Target node '{}' does not have input port {}",
          conn.target.0, conn.target.1
        )));
      }
    }

    Ok(())
  }

  /// Creates channels for all connections in the graph.
  ///
  /// This method creates bounded channels for routing data between nodes.
  /// Each connection gets a channel pair (sender, receiver) with configurable
  /// buffer size for backpressure control.
  ///
  /// # Channel Routing Strategy
  ///
  /// Channels are created for each connection in the graph:
  /// - Source node output port -> Channel sender
  /// - Channel receiver -> Target node input port
  ///
  /// Bounded channels provide automatic backpressure: when the buffer is full,
  /// senders will block until space is available, preventing memory issues.
  ///
  /// # Returns
  ///
  /// `Ok(())` if channels were created successfully, `Err(ExecutionError)` otherwise.
  fn create_channels(&mut self) -> Result<(), ExecutionError> {
    self.create_channels_with_buffer_size(1024)
  }

  /// Creates channels with a custom buffer size.
  ///
  /// # Arguments
  ///
  /// * `buffer_size` - The buffer size for each channel (number of items)
  ///
  /// # Returns
  ///
  /// `Ok(())` if channels were created successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Note
  ///
  /// Smaller buffer sizes provide tighter backpressure control but may reduce
  /// throughput. Larger buffer sizes improve throughput but use more memory.
  pub fn create_channels_with_buffer_size(
    &mut self,
    buffer_size: usize,
  ) -> Result<(), ExecutionError> {
    for conn in self.graph.get_connections() {
      let (sender, receiver) = mpsc::channel(buffer_size);

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

  /// Returns a reference to the channel sender for a given node and port.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  /// * `port_index` - The output port index
  ///
  /// # Returns
  ///
  /// `Some(&mpsc::Sender<Vec<u8>>)` if the channel exists, `None` otherwise.
  ///
  /// # Note
  ///
  /// This method is used by node tasks to send data to downstream nodes.
  /// The sender will block when the channel buffer is full, providing
  /// automatic backpressure.
  pub fn get_channel_sender(
    &self,
    node_name: &str,
    port_index: usize,
  ) -> Option<&mpsc::Sender<Vec<u8>>> {
    self
      .channel_senders
      .get(&(node_name.to_string(), port_index))
  }

  /// Returns a mutable reference to the channel receiver for a given node and port.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  /// * `port_index` - The input port index
  ///
  /// # Returns
  ///
  /// `Some(&mut mpsc::Receiver<Vec<u8>>)` if the channel exists, `None` otherwise.
  ///
  /// # Note
  ///
  /// This method is used by node tasks to receive data from upstream nodes.
  pub fn get_channel_receiver(
    &mut self,
    node_name: &str,
    port_index: usize,
  ) -> Option<&mut mpsc::Receiver<Vec<u8>>> {
    self
      .channel_receivers
      .get_mut(&(node_name.to_string(), port_index))
  }

  /// Returns the number of channels created for routing.
  ///
  /// # Returns
  ///
  /// The number of channel pairs (one per connection).
  pub fn channel_count(&self) -> usize {
    self.channel_senders.len()
  }

  /// Checks if a channel exists for a given node and port.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  /// * `port_index` - The port index
  /// * `is_output` - `true` for output port (sender), `false` for input port (receiver)
  ///
  /// # Returns
  ///
  /// `true` if the channel exists, `false` otherwise.
  pub fn has_channel(&self, node_name: &str, port_index: usize, is_output: bool) -> bool {
    let key = (node_name.to_string(), port_index);
    if is_output {
      self.channel_senders.contains_key(&key)
    } else {
      self.channel_receivers.contains_key(&key)
    }
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
  use crate::{GraphBuilder, ProducerNode};
  use streamweave_producer_vec::VecProducer;

  #[tokio::test]
  async fn test_graph_executor_creation() {
    let graph = Graph::new();
    let executor = graph.executor();

    assert_eq!(executor.state(), ExecutionState::Stopped);
    assert!(!executor.is_running());
  }

  #[tokio::test]
  async fn test_graph_executor_start_stop() {
    // Create a graph with at least one node
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();
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
    // Create a graph with at least one node
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();
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
    // Create a graph with at least one node
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();
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
    // Create a graph with at least one node
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();
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

  #[tokio::test]
  async fn test_graph_executor_channel_creation() {
    use crate::{GraphBuilder, ProducerNode};
    use streamweave_producer_vec::VecProducer;

    // Create a simple graph with one node (no connections yet)
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();

    // Create channels (even though there are no connections, this should succeed)
    assert!(executor.create_channels_with_buffer_size(64).is_ok());
    assert_eq!(executor.channel_count(), 0);
  }

  #[tokio::test]
  async fn test_graph_executor_channel_helpers() {
    use crate::{GraphBuilder, ProducerNode};
    use streamweave_producer_vec::VecProducer;

    // Create a simple graph
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();

    // No channels exist yet
    assert!(!executor.has_channel("source", 0, true));
    assert_eq!(executor.channel_count(), 0);
    assert!(executor.get_channel_sender("source", 0).is_none());
    assert!(executor.get_channel_receiver("source", 0).is_none());
  }

  #[tokio::test]
  async fn test_graph_executor_lifecycle_errors() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Initially no errors
    assert!(executor.errors().is_empty());

    // Clear errors (should be no-op)
    executor.clear_errors();
    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_graph_executor_shutdown_timeout() {
    let graph = Graph::new();
    let executor = graph.executor();

    // Default timeout is 30 seconds
    assert_eq!(executor.shutdown_timeout(), Duration::from_secs(30));

    // Create executor with custom timeout (need new graph since previous one was moved)
    let graph2 = Graph::new();
    let custom_timeout = Duration::from_secs(60);
    let mut executor = GraphExecutor::with_shutdown_timeout(graph2, custom_timeout);
    assert_eq!(executor.shutdown_timeout(), custom_timeout);

    // Set new timeout
    executor.set_shutdown_timeout(Duration::from_secs(10));
    assert_eq!(executor.shutdown_timeout(), Duration::from_secs(10));
  }

  #[tokio::test]
  async fn test_graph_executor_stop_immediate() {
    // Create a graph with at least one node
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();
    let mut executor = graph.executor();

    // Start execution
    assert!(executor.start().await.is_ok());

    // Stop immediately
    assert!(executor.stop_immediate().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Stopped);
  }
}
