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
//! # Message-Based Data Flow
//!
//! All data flowing through the graph is wrapped in `Message<T>` to enable
//! end-to-end traceability and metadata preservation. The execution engine
//! creates type-erased channels (`TypeErasedSender`/`TypeErasedReceiver`)
//! that carry `ChannelItem` instances, which can contain:
//! - `ChannelItem::Arc`: Zero-copy `Arc<Message<T>>` for in-process execution
//! - `ChannelItem::Arc`: `Arc<Message<T>>` for zero-copy in-process execution
//!
//! Nodes automatically wrap/unwrap `Message<T>` as needed:
//! - Producer nodes wrap outputs in `Message<T>` before sending
//! - Transformer nodes unwrap `Message<T::Input>`, transform, then wrap `Message<T::Output>`
//! - Consumer nodes unwrap `Message<C::Input>` before consuming
//!
//! # Execution Flow
//!
//! 1. **Initialization**: Create channels for each connection in the graph
//! 2. **Node Spawning**: Spawn tasks for each node type:
//!    - Producer nodes: Start producing, wrap in `Message<T>`, send to output channels
//!    - Transformer nodes: Receive `Message<T::Input>` from input channels, unwrap, transform, wrap `Message<T::Output>`, send to output channels
//!    - Consumer nodes: Receive `Message<C::Input>` from input channels, unwrap, and consume
//! 3. **Execution**: Nodes run concurrently, with `Message<T>` flowing through channels
//! 4. **Shutdown**: Gracefully stop all nodes and clean up resources

use super::channels::{TypeErasedReceiver, TypeErasedSender};
use super::graph::Graph;
use super::shared_memory_channel::SharedMemoryChannel;
use super::zero_copy::ArcPool;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio::time::timeout;

/// Error type for graph execution.
///
/// All errors in the graph execution system work with `Message<T>` types.
/// When errors occur during message processing, relevant error variants
/// include optional message IDs to enable traceability back to the source
/// message that caused the error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
  /// Node execution failed
  NodeExecutionFailed {
    /// Node name
    node: String,
    /// Error message
    reason: String,
    /// Optional message ID if error occurred while processing a specific message
    message_id: Option<String>,
  },
  /// Connection error during execution
  ConnectionError {
    /// Source node and port
    source: (String, String),
    /// Target node and port
    target: (String, String),
    /// Error reason
    reason: String,
    /// Optional message ID if error occurred while routing a specific message
    message_id: Option<String>,
  },
  /// Channel error during execution
  ChannelError {
    /// Node name where error occurred
    node: String,
    /// Port name where error occurred
    port: String,
    /// Whether this is an input or output port
    is_input: bool,
    /// Error reason
    reason: String,
    /// Optional message ID if error occurred while processing a specific message
    message_id: Option<String>,
  },
  /// Serialization error during execution
  SerializationError {
    /// Node name where error occurred
    node: String,
    /// Whether this is serialization or deserialization
    is_deserialization: bool,
    /// Error details
    reason: String,
    /// Optional message ID if error occurred while serializing/deserializing a specific message
    message_id: Option<String>,
  },
  /// Stream error during execution
  StreamError {
    /// Node name where error occurred
    node: String,
    /// Error reason
    reason: String,
    /// Optional message ID if error occurred while processing a specific message
    message_id: Option<String>,
  },
  /// Graph execution was cancelled
  Cancelled,
  /// Graph execution failed with an error
  ExecutionFailed(String),
  /// Invalid graph topology
  InvalidTopology(String),
  /// Shutdown timeout exceeded
  ShutdownTimeout {
    /// Timeout duration in seconds
    timeout_secs: u64,
    /// Nodes that failed to shutdown
    nodes: Vec<String>,
  },
  /// Pause/resume operation failed
  LifecycleError {
    /// Operation that failed
    operation: String,
    /// Current state
    current_state: ExecutionState,
    /// Error reason
    reason: String,
  },
  /// Other error (catch-all for miscellaneous errors)
  Other(String),
  /// Channel creation failed
  ChannelCreationError {
    /// Node name where error occurred
    node: String,
    /// Error reason
    reason: String,
  },
}

impl std::fmt::Display for ExecutionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ExecutionError::NodeExecutionFailed {
        node,
        reason,
        message_id,
      } => {
        if let Some(msg_id) = message_id {
          write!(
            f,
            "Node '{}' execution failed (message ID: {}): {}",
            node, msg_id, reason
          )
        } else {
          write!(f, "Node '{}' execution failed: {}", node, reason)
        }
      }
      ExecutionError::ConnectionError {
        source,
        target,
        reason,
        message_id,
      } => {
        if let Some(msg_id) = message_id {
          write!(
            f,
            "Connection error from {}:{} to {}:{} (message ID: {}): {}",
            source.0, source.1, target.0, target.1, msg_id, reason
          )
        } else {
          write!(
            f,
            "Connection error from {}:{} to {}:{}: {}",
            source.0, source.1, target.0, target.1, reason
          )
        }
      }
      ExecutionError::ChannelError {
        node,
        port,
        is_input,
        reason,
        message_id,
      } => {
        let port_type = if *is_input { "input" } else { "output" };
        if let Some(msg_id) = message_id {
          write!(
            f,
            "Channel error on node '{}' {} port {} (message ID: {}): {}",
            node, port_type, port, msg_id, reason
          )
        } else {
          write!(
            f,
            "Channel error on node '{}' {} port {}: {}",
            node, port_type, port, reason
          )
        }
      }
      ExecutionError::SerializationError {
        node,
        is_deserialization,
        reason,
        message_id,
      } => {
        let op_type = if *is_deserialization {
          "deserialization"
        } else {
          "serialization"
        };
        if let Some(msg_id) = message_id {
          write!(
            f,
            "{} error on node '{}' (message ID: {}): {}",
            op_type, node, msg_id, reason
          )
        } else {
          write!(f, "{} error on node '{}': {}", op_type, node, reason)
        }
      }
      ExecutionError::StreamError {
        node,
        reason,
        message_id,
      } => {
        if let Some(msg_id) = message_id {
          write!(
            f,
            "Stream error on node '{}' (message ID: {}): {}",
            node, msg_id, reason
          )
        } else {
          write!(f, "Stream error on node '{}': {}", node, reason)
        }
      }
      ExecutionError::Cancelled => write!(f, "Graph execution was cancelled"),
      ExecutionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
      ExecutionError::InvalidTopology(msg) => write!(f, "Invalid graph topology: {}", msg),
      ExecutionError::ShutdownTimeout {
        timeout_secs,
        nodes,
      } => {
        write!(
          f,
          "Shutdown timeout ({}) exceeded. Nodes that failed to shutdown: {:?}",
          timeout_secs, nodes
        )
      }
      ExecutionError::LifecycleError {
        operation,
        current_state,
        reason,
      } => {
        write!(
          f,
          "Lifecycle operation '{}' failed in state {:?}: {}",
          operation, current_state, reason
        )
      }
      ExecutionError::ChannelCreationError { node, reason } => {
        write!(f, "Channel creation failed for node '{}': {}", node, reason)
      }
      ExecutionError::Other(msg) => write!(f, "{}", msg),
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
/// use crate::graph::{Graph, GraphBuilder, GraphExecution};
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
  /// Key: (node_name, port_name)
  /// Uses type-erased channels that can hold either Bytes (distributed) or `Arc<T>` (in-process)
  channel_senders: HashMap<(String, String), TypeErasedSender>,
  /// Channel receivers for routing data between nodes
  /// Key: (node_name, port_name)
  /// Uses type-erased channels that can hold either Bytes (distributed) or `Arc<T>` (in-process)
  channel_receivers: HashMap<(String, String), TypeErasedReceiver>,
  /// Execution state
  state: ExecutionState,
  /// Pause signal shared across all node tasks
  /// When true, nodes should pause execution
  pause_signal: Arc<RwLock<bool>>,
  /// Errors collected during execution
  execution_errors: Vec<ExecutionError>,
  /// Shutdown timeout duration
  shutdown_timeout: Duration,
  /// Optional Arc pool for high-performance fan-out scenarios
  /// When provided, this pool will be used to reduce allocation overhead
  /// in fan-out operations where multiple nodes receive the same data.
  arc_pool: Option<ArcPool<Bytes>>,
  /// Whether to use shared memory for ultra-high performance mode
  use_shared_memory: bool,
  /// Shared memory channels for ultra-high performance mode
  /// Key: (node_name, port_name) for source nodes
  /// Value: Shared memory channel for that connection
  shared_memory_channels: HashMap<(String, String), SharedMemoryChannel>,
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
  /// use crate::graph::{Graph, GraphExecution};
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
      arc_pool: None,
      use_shared_memory: false, // Default to not using shared memory
      shared_memory_channels: HashMap::new(),
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
      arc_pool: None,
      use_shared_memory: false, // Default to not using shared memory
      shared_memory_channels: HashMap::new(),
    }
  }

  /// Sets an Arc pool for high-performance fan-out scenarios.
  ///
  /// When provided, this pool will be used to reduce allocation overhead
  /// in fan-out operations where multiple nodes receive the same data.
  /// The pool maintains reusable `Arc` instances to avoid frequent allocations.
  ///
  /// # Arguments
  ///
  /// * `pool` - An `ArcPool<Bytes>` to use for fan-out operations
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::graph::{Graph, GraphExecution};
  /// use crate::graph::ArcPool;
  ///
  /// let graph = Graph::new();
  /// let pool = ArcPool::<bytes::Bytes>::new(100);
  /// let executor = graph.executor().with_arc_pool(pool);
  /// ```
  #[must_use]
  pub fn with_arc_pool(mut self, pool: ArcPool<Bytes>) -> Self {
    self.arc_pool = Some(pool);
    self
  }

  /// Sets whether to use shared memory for ultra-high performance mode.
  ///
  /// When enabled, the executor will use shared memory channels for data
  /// transfer between nodes, which can provide better performance for
  /// high-throughput scenarios.
  ///
  /// # Arguments
  ///
  /// * `use_shared_memory` - Whether to use shared memory channels
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::graph::{Graph, GraphExecution};
  ///
  /// let graph = Graph::new();
  /// let executor = graph.executor().with_use_shared_memory(true);
  /// ```
  #[must_use]
  pub fn with_use_shared_memory(mut self, use_shared_memory: bool) -> Self {
    self.use_shared_memory = use_shared_memory;
    self
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
  /// use crate::graph::{Graph, GraphExecution};
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
    self.execute_in_process(self.use_shared_memory).await
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
  /// use crate::graph::{Graph, GraphExecution};
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

    // Collect node names before moving handles into the closure
    let node_names: Vec<String> = handles.iter().map(|(name, _)| name.clone()).collect();

    let shutdown_result = timeout(self.shutdown_timeout, async {
      let mut errors = Vec::new();
      for (node_name, handle) in handles {
        match handle.await {
          Ok(Ok(())) => {
            // Task completed successfully
          }
          Ok(Err(e)) => {
            // Task returned an error - preserve the original error type
            errors.push(e);
          }
          Err(e) => {
            // Task was cancelled or panicked
            if e.is_cancelled() {
              // Expected during shutdown - don't report as error
            } else if e.is_panic() {
              errors.push(ExecutionError::NodeExecutionFailed {
                node: node_name.clone(),
                reason: format!("Task panicked: {}", e),
                message_id: None,
              });
            } else {
              // Other join errors
              errors.push(ExecutionError::NodeExecutionFailed {
                node: node_name.clone(),
                reason: format!("Task join error: {}", e),
                message_id: None,
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
        // Timeout occurred - use the node names we collected earlier
        return Err(ExecutionError::ShutdownTimeout {
          timeout_secs: self.shutdown_timeout.as_secs(),
          nodes: node_names,
        });
      }
    }

    // Clean up remaining resources
    self.channel_receivers.clear();

    // Cleanup shared memory channels
    // The shared_memory crate will automatically cleanup segments when
    // the last Arc<Shmem> is dropped, but we explicitly clear here to
    // ensure all references are dropped promptly.
    for channel in self.shared_memory_channels.values() {
      channel.cleanup();
    }
    self.shared_memory_channels.clear();

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

    // Cleanup shared memory channels
    for channel in self.shared_memory_channels.values() {
      channel.cleanup();
    }
    self.shared_memory_channels.clear();

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
  /// use crate::graph::{Graph, GraphExecution};
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
      return Err(ExecutionError::LifecycleError {
        operation: "pause".to_string(),
        current_state: self.state,
        reason: format!("Cannot pause graph in {:?} state", self.state),
      });
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
  /// use crate::graph::{Graph, GraphExecution};
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
      return Err(ExecutionError::LifecycleError {
        operation: "resume".to_string(),
        current_state: self.state,
        reason: format!("Cannot resume graph in {:?} state", self.state),
      });
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
  #[allow(clippy::result_large_err)]
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

      // Validate port names exist
      if let Some(source_node) = self.graph.get_node(&conn.source.0)
        && !source_node.has_output_port(&conn.source.1)
      {
        return Err(ExecutionError::InvalidTopology(format!(
          "Source node '{}' does not have output port '{}'",
          conn.source.0, conn.source.1
        )));
      }

      if let Some(target_node) = self.graph.get_node(&conn.target.0)
        && !target_node.has_input_port(&conn.target.1)
      {
        return Err(ExecutionError::InvalidTopology(format!(
          "Target node '{}' does not have input port '{}'",
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
  /// All data flowing through channels is wrapped in `Message<T>`:
  /// - In-process mode: `ChannelItem::Arc` contains `Arc<Message<T>>`
  /// - Shared memory mode: `ChannelItem::SharedMemory` contains a reference to `Message<T>` in shared memory
  ///
  /// Bounded channels provide automatic backpressure: when the buffer is full,
  /// senders will block until space is available, preventing memory issues.
  ///
  /// # Returns
  ///
  /// `Ok(())` if channels were created successfully, `Err(ExecutionError)` otherwise.
  #[allow(clippy::result_large_err)]
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
  #[allow(clippy::result_large_err)]
  pub fn create_channels_with_buffer_size(
    &mut self,
    buffer_size: usize,
  ) -> Result<(), ExecutionError> {
    let use_shared_memory = self.use_shared_memory;

    for conn in self.graph.get_connections() {
      // ConnectionInfo now stores port names directly
      let source_port_name = &conn.source.1;
      let target_port_name = &conn.target.1;

      if use_shared_memory {
        // Create shared memory channel
        let segment_id = format!(
          "streamweave_{}_{}_{}_{}",
          conn.source.0, source_port_name, conn.target.0, target_port_name
        );

        // Create shared memory channel with comprehensive error handling
        let shared_channel = match SharedMemoryChannel::new(&segment_id, buffer_size) {
          Ok(channel) => channel,
          Err(e) => {
            // Provide detailed error message for debugging
            return Err(ExecutionError::ChannelCreationError {
              node: conn.source.0.clone(),
              reason: format!(
                "Failed to create shared memory channel '{}': {}. \
                 Possible causes: insufficient permissions, existing segment with same name, \
                 or system limits (check /proc/sys/kernel/shmmax on Linux).",
                segment_id, e
              ),
            });
          }
        };

        // Store shared memory channel for both source and target
        // Source uses it to send, target uses it to receive
        self.shared_memory_channels.insert(
          (conn.source.0.clone(), source_port_name.clone()),
          shared_channel.clone(),
        );
        self.shared_memory_channels.insert(
          (conn.target.0.clone(), target_port_name.clone()),
          shared_channel,
        );

        // Still create regular channels for sending SharedMemoryRef
        let (sender, receiver): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(buffer_size);

        self
          .channel_senders
          .insert((conn.source.0.clone(), source_port_name.clone()), sender);

        self
          .channel_receivers
          .insert((conn.target.0.clone(), target_port_name.clone()), receiver);
      } else {
        // Create type-erased channels that can hold either Bytes (serialized Message<T>)
        // or Arc<Message<T>>. Nodes will determine which variant to use based on ExecutionMode
        let (sender, receiver): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(buffer_size);

        // Store the sender
        self
          .channel_senders
          .insert((conn.source.0.clone(), source_port_name.clone()), sender);

        // Store receiver for target node's input port
        self
          .channel_receivers
          .insert((conn.target.0.clone(), target_port_name.clone()), receiver);
      }
    }

    Ok(())
  }

  /// Executes the graph in in-process zero-copy mode.
  ///
  /// This method implements zero-copy execution by passing data directly
  /// between nodes without serialization. For fan-out scenarios, data is
  /// shared using `Arc` to avoid copying.
  ///
  /// # Arguments
  ///
  /// * `use_shared_memory` - Whether to use shared memory for ultra-high
  ///   performance scenarios (future optimization)
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution started successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Zero-Copy Semantics
  ///
  /// - Data is passed directly between nodes without serialization
  /// - All data is wrapped in `Message<T>` for traceability
  /// - Fan-out scenarios use `Arc<Message<T>>` for zero-copy sharing
  /// - Fan-in scenarios merge streams directly
  /// - No serialization/deserialization overhead
  ///
  /// # Note
  ///
  /// This method requires that all nodes are in the same process and that
  /// the graph topology allows for direct stream connections. The actual
  /// zero-copy execution is implemented in the node execution code, which
  /// uses `Arc<Message<T>>` channels instead of Bytes channels when in in-process mode.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::graph::{Graph, GraphExecution};
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// // Execute in zero-copy in-process mode
  /// executor.execute_in_process(false).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn execute_in_process(
    &mut self,
    use_shared_memory: bool,
  ) -> Result<(), ExecutionError> {
    if self.state == ExecutionState::Running {
      return Err(ExecutionError::ExecutionFailed(
        "Graph is already running".to_string(),
      ));
    }

    // Set shared memory flag
    self.use_shared_memory = use_shared_memory;

    // Validate graph topology
    self.validate_topology()?;

    // For in-process execution, nodes will use Arc<T> channels internally
    // The node execution code checks ExecutionMode and creates appropriate channels
    // We still create Bytes channels here as a fallback, but nodes will use
    // direct stream connections when ExecutionMode::InProcess is detected
    self.create_channels()?;

    // Spawn tasks for each node
    // In in-process mode, nodes will use direct stream connections and Arc for fan-out
    for node_name in self.graph.node_names() {
      if let Some(node) = self.graph.get_node(&node_name) {
        // Collect input channels for this node
        let mut input_channels = HashMap::new();
        let parents = self.graph.get_parents(&node_name);
        for (parent_name, parent_port_name) in parents {
          // Find which input port this connection targets
          for conn in self.graph.get_connections() {
            if conn.source.0 == parent_name
              && conn.source.1 == parent_port_name
              && conn.target.0 == node_name
            {
              // ConnectionInfo now stores port names directly
              let target_port_name = conn.target.1.clone();
              let source_port_name = conn.source.1.clone();

              let key = (parent_name.to_string(), source_port_name);
              if let Some(receiver) = self.channel_receivers.remove(&key) {
                input_channels.insert(target_port_name, receiver);
              }
              break;
            }
          }
        }

        // Collect output channels for this node
        let mut output_channels = HashMap::new();
        let children = self.graph.get_children(&node_name);
        for (child_name, child_port_name) in children {
          // Find which output port this connection comes from
          for conn in self.graph.get_connections() {
            if conn.source.0 == node_name
              && conn.target.0 == child_name
              && conn.target.1 == child_port_name
            {
              // ConnectionInfo now stores port names directly
              let source_port_name = conn.source.1.clone();

              let key = (node_name.to_string(), source_port_name.clone());
              if let Some(sender) = self.channel_senders.get(&key).cloned() {
                output_channels.insert(source_port_name, sender);
              }
              break;
            }
          }
        }

        // Spawn execution task
        // In in-process mode, nodes will use Arc<Message<T>> channels and direct stream passing
        // The node execution code checks ExecutionMode and uses appropriate channel types
        // All data is wrapped in Message<T> for end-to-end traceability
        // Clone arc_pool if available (wrap in Arc for sharing across tasks)
        let arc_pool_clone = self.arc_pool.as_ref().map(|p| Arc::new(p.clone()));

        if let Some(handle) = node.spawn_execution_task(
          input_channels,
          output_channels,
          self.pause_signal.clone(),
          use_shared_memory,
          arc_pool_clone,
        ) {
          // Insert handle directly
          self.node_handles.insert(node_name.to_string(), handle);
        } else {
          return Err(ExecutionError::NodeExecutionFailed {
            node: node_name.to_string(),
            reason: "Node does not support execution".to_string(),
            message_id: None,
          });
        }
      }
    }

    self.state = ExecutionState::Running;
    Ok(())
  }

  /// Returns a reference to the channel sender for a given node and port.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  /// * `port_name` - The output port name
  ///
  /// # Returns
  ///
  /// `Some(&TypeErasedSender)` if the channel exists, `None` otherwise.
  ///
  /// # Note
  ///
  /// This method is used by node tasks to send data to downstream nodes.
  /// The sender will block when the channel buffer is full, providing
  /// automatic backpressure. Nodes should wrap items in `Message<T>` and then
  /// send as `ChannelItem::Arc` (`Arc<Message<T>>`) or `ChannelItem::SharedMemory`
  /// (shared memory reference) based on `ExecutionMode`.
  pub fn get_channel_sender(&self, node_name: &str, port_name: &str) -> Option<&TypeErasedSender> {
    self
      .channel_senders
      .get(&(node_name.to_string(), port_name.to_string()))
  }

  /// Returns a mutable reference to the channel receiver for a given node and port.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  /// * `port_name` - The input port name
  ///
  /// # Returns
  ///
  /// `Some(&mut TypeErasedReceiver)` if the channel exists, `None` otherwise.
  ///
  /// # Note
  ///
  /// This method is used by node tasks to receive data from upstream nodes.
  /// Nodes should extract `ChannelItem::Arc` (downcast to `Arc<Message<T>>`) or
  /// `ChannelItem::SharedMemory` (access shared memory reference) based on `ExecutionMode`,
  /// then access the `Message<T>` to get the payload.
  pub fn get_channel_receiver(
    &mut self,
    node_name: &str,
    port_name: &str,
  ) -> Option<&mut TypeErasedReceiver> {
    self
      .channel_receivers
      .get_mut(&(node_name.to_string(), port_name.to_string()))
  }

  /// Returns a reference to the shared memory channel for a given node and port.
  ///
  /// # Arguments
  ///
  /// * `node_name` - The name of the node
  /// * `port_name` - The port name
  ///
  /// # Returns
  ///
  /// `Some(&SharedMemoryChannel)` if the channel exists, `None` otherwise.
  ///
  /// # Note
  ///
  /// This method is used by node tasks to access shared memory channels
  /// when `use_shared_memory` is enabled in `ExecutionMode::InProcess`.
  pub fn get_shared_memory_channel(
    &self,
    node_name: &str,
    port_name: &str,
  ) -> Option<&SharedMemoryChannel> {
    self
      .shared_memory_channels
      .get(&(node_name.to_string(), port_name.to_string()))
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
  /// * `port_name` - The port name
  /// * `is_output` - `true` for output port (sender), `false` for input port (receiver)
  ///
  /// # Returns
  ///
  /// `true` if the channel exists, `false` otherwise.
  pub fn has_channel(&self, node_name: &str, port_name: &str, is_output: bool) -> bool {
    let key = (node_name.to_string(), port_name.to_string());
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
/// use crate::graph::{Graph, GraphExecution};
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
