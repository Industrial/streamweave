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
//! - `ChannelItem::Bytes`: Serialized `Message<T>` for distributed execution
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

use super::batching::BatchingChannel;
use super::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use super::graph::Graph;
use super::shared_memory_channel::SharedMemoryChannel;
use super::throughput::ThroughputMonitor;
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
  /// Channel creation failed
  ChannelCreationError {
    /// Node name where error occurred
    node: String,
    /// Error reason
    reason: String,
  },
  /// Compression or decompression error
  CompressionError {
    /// Node name where error occurred
    node: String,
    /// Whether this is compression or decompression
    is_compression: bool,
    /// Error details
    reason: String,
    /// Whether the error is due to corrupted data
    is_corrupted: bool,
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
      ExecutionError::CompressionError {
        node,
        is_compression,
        reason,
        is_corrupted,
      } => {
        let op = if *is_compression {
          "compression"
        } else {
          "decompression"
        };
        let corrupted_msg = if *is_corrupted {
          " (corrupted data)"
        } else {
          ""
        };
        write!(
          f,
          "{} error on node '{}': {}{}",
          op, node, reason, corrupted_msg
        )
      }
    }
  }
}

impl std::error::Error for ExecutionError {}

/// Execution mode for graph execution.
///
/// This enum defines how a graph should be executed, supporting both
/// in-process zero-copy execution and distributed serialized execution.
///
/// # Variants
///
/// - `InProcess`: Zero-copy in-process execution with optional shared memory
/// - `Distributed`: Serialized execution for distributed scenarios
/// - `Hybrid`: Adaptive execution that switches between modes based on load
///
/// # Example
///
/// ```rust
/// use streamweave::graph::execution::ExecutionMode;
///
/// // In-process zero-copy execution
/// let mode = ExecutionMode::InProcess { use_shared_memory: false };
///
/// // Distributed execution with a serializer
/// let serializer: Box<dyn Serializer> = /* your serializer implementation */;
/// let mode = ExecutionMode::Distributed {
///     serializer,
///     compression: None,
///     batching: None,
/// };
/// ```
#[derive(Debug, Clone)]
pub enum ExecutionMode {
  /// In-process zero-copy execution mode.
  ///
  /// This mode eliminates serialization overhead by passing data directly
  /// between nodes in the same process. For fan-out scenarios, data is
  /// shared using `Arc` to avoid copying.
  ///
  /// # Fields
  ///
  /// * `use_shared_memory` - Whether to use shared memory for ultra-high
  ///   performance scenarios (future optimization)
  InProcess {
    /// Whether to use shared memory for data sharing
    use_shared_memory: bool,
  },
  /// Distributed serialized execution mode.
  ///
  /// This mode serializes data for transmission between nodes, enabling
  /// distributed execution across processes or machines. Supports optional
  /// compression and batching for efficiency.
  ///
  /// # Fields
  ///
  /// * `serializer` - The serializer to use for data serialization
  /// * `compression` - Optional compression algorithm for serialized data
  /// * `batching` - Optional batching configuration for grouping items
  Distributed {
    /// Serializer for data serialization
    serializer: super::serialization::JsonSerializer,
    /// Optional compression algorithm
    compression: Option<CompressionAlgorithm>,
    /// Optional batching configuration
    batching: Option<BatchConfig>,
  },
  /// Hybrid execution mode that adapts between in-process and distributed.
  ///
  /// This mode starts in in-process mode and switches to distributed mode
  /// when the local threshold is exceeded, enabling adaptive performance
  /// optimization based on load.
  ///
  /// # Fields
  ///
  /// * `local_threshold` - Threshold for switching from in-process to distributed
  /// * `serializer` - Serializer to use when in distributed mode
  Hybrid {
    /// Threshold for switching to distributed mode
    local_threshold: usize,
    /// Serializer for distributed mode
    serializer: super::serialization::JsonSerializer,
  },
}

/// Compression algorithm for distributed execution.
///
/// This enum defines compression algorithms that can be used to compress
/// serialized data in distributed execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
  /// Gzip compression with configurable level (1-9)
  Gzip {
    /// Compression level (1-9, default 6)
    level: u32,
  },
  /// Zstandard compression with configurable level (1-22)
  Zstd {
    /// Compression level (1-22, default 3)
    level: u32,
  },
}

impl CompressionAlgorithm {
  /// Create a new Gzip compression algorithm with the specified level.
  ///
  /// # Arguments
  ///
  /// * `level` - Compression level (1-9). Defaults to 6 if out of range.
  ///
  /// # Returns
  ///
  /// A `CompressionAlgorithm::Gzip` variant with the specified level.
  pub fn gzip(level: u32) -> Self {
    Self::Gzip {
      level: level.clamp(1, 9),
    }
  }

  /// Create a new Gzip compression algorithm with default level (6).
  pub fn gzip_default() -> Self {
    Self::Gzip { level: 6 }
  }

  /// Create a new Zstd compression algorithm with the specified level.
  ///
  /// # Arguments
  ///
  /// * `level` - Compression level (1-22). Defaults to 3 if out of range.
  ///
  /// # Returns
  ///
  /// A `CompressionAlgorithm::Zstd` variant with the specified level.
  pub fn zstd(level: u32) -> Self {
    Self::Zstd {
      level: level.clamp(1, 22),
    }
  }

  /// Create a new Zstd compression algorithm with default level (3).
  pub fn zstd_default() -> Self {
    Self::Zstd { level: 3 }
  }

  /// Get the compression level for this algorithm.
  ///
  /// # Returns
  ///
  /// The compression level (1-9 for gzip, 1-22 for zstd).
  pub fn level(&self) -> u32 {
    match self {
      CompressionAlgorithm::Gzip { level } => *level,
      CompressionAlgorithm::Zstd { level } => *level,
    }
  }
}

/// Batching configuration for distributed execution.
///
/// This structure defines how items should be batched together before
/// serialization and transmission in distributed execution mode.
#[derive(Debug, Clone)]
pub struct BatchConfig {
  /// Maximum number of items per batch
  pub batch_size: usize,
  /// Maximum time to wait before sending a batch (in milliseconds)
  pub batch_timeout_ms: u64,
}

impl BatchConfig {
  /// Create a new `BatchConfig` with the given batch size and timeout.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Maximum number of items per batch (must be > 0)
  /// * `batch_timeout_ms` - Maximum time to wait before sending a batch (in milliseconds, must be > 0)
  ///
  /// # Returns
  ///
  /// A new `BatchConfig` instance
  ///
  /// # Panics
  ///
  /// Panics if `batch_size` is 0 or `batch_timeout_ms` is 0.
  #[must_use]
  pub fn new(batch_size: usize, batch_timeout_ms: u64) -> Self {
    Self::validate(batch_size, batch_timeout_ms).expect("Invalid batch configuration");
    Self {
      batch_size,
      batch_timeout_ms,
    }
  }

  /// Create a new `BatchConfig` with validation.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Maximum number of items per batch
  /// * `batch_timeout_ms` - Maximum time to wait before sending a batch (in milliseconds)
  ///
  /// # Returns
  ///
  /// `Ok(BatchConfig)` if the configuration is valid, `Err(String)` otherwise.
  pub fn try_new(batch_size: usize, batch_timeout_ms: u64) -> Result<Self, String> {
    Self::validate(batch_size, batch_timeout_ms)?;
    Ok(Self {
      batch_size,
      batch_timeout_ms,
    })
  }

  /// Validate batch configuration parameters.
  ///
  /// # Arguments
  ///
  /// * `batch_size` - Maximum number of items per batch
  /// * `batch_timeout_ms` - Maximum time to wait before sending a batch (in milliseconds)
  ///
  /// # Returns
  ///
  /// `Ok(())` if valid, `Err(String)` with error message otherwise.
  pub fn validate(batch_size: usize, batch_timeout_ms: u64) -> Result<(), String> {
    if batch_size == 0 {
      return Err("batch_size must be greater than 0".to_string());
    }

    if batch_timeout_ms == 0 {
      return Err("batch_timeout_ms must be greater than 0".to_string());
    }

    // Reasonable upper limit: 1 hour (3,600,000 ms)
    const MAX_TIMEOUT_MS: u64 = 3_600_000;
    if batch_timeout_ms > MAX_TIMEOUT_MS {
      return Err(format!(
        "batch_timeout_ms ({}) exceeds maximum allowed value ({} ms = 1 hour)",
        batch_timeout_ms, MAX_TIMEOUT_MS
      ));
    }

    // Reasonable upper limit for batch size: 1 million items
    const MAX_BATCH_SIZE: usize = 1_000_000;
    if batch_size > MAX_BATCH_SIZE {
      return Err(format!(
        "batch_size ({}) exceeds maximum allowed value ({})",
        batch_size, MAX_BATCH_SIZE
      ));
    }

    Ok(())
  }
}

/// Metrics for tracking mode switches in hybrid execution mode.
///
/// This structure tracks when and why the execution mode switches between
/// in-process and distributed modes in hybrid execution.
#[derive(Debug, Clone)]
pub struct ModeSwitchMetrics {
  /// Number of mode switches that have occurred
  pub switch_count: usize,
  /// Throughput values that triggered mode switches
  pub switch_reasons: Vec<f64>,
  /// Current execution mode name
  pub current_mode: Option<String>,
}

impl ModeSwitchMetrics {
  /// Create a new `ModeSwitchMetrics` instance.
  ///
  /// # Returns
  ///
  /// A new `ModeSwitchMetrics` with no switches recorded.
  #[must_use]
  pub fn new() -> Self {
    Self {
      switch_count: 0,
      switch_reasons: Vec::new(),
      current_mode: None,
    }
  }

  /// Record a mode switch event.
  ///
  /// # Arguments
  ///
  /// * `throughput` - The throughput value that triggered the switch
  /// * `from` - The mode we're switching from
  /// * `to` - The mode we're switching to
  pub fn record_switch(&mut self, throughput: f64, _from: &str, to: &str) {
    self.switch_count += 1;
    self.switch_reasons.push(throughput);
    self.current_mode = Some(to.to_string());
  }

  /// Get a summary string of the mode switch metrics.
  ///
  /// # Returns
  ///
  /// A formatted string summarizing the mode switch metrics.
  pub fn summary(&self) -> String {
    format!(
      "switches: {}, current_mode: {:?}, avg_throughput: {:.2}",
      self.switch_count,
      self.current_mode,
      if self.switch_reasons.is_empty() {
        0.0
      } else {
        self.switch_reasons.iter().sum::<f64>() / self.switch_reasons.len() as f64
      }
    )
  }
}

impl Default for ModeSwitchMetrics {
  fn default() -> Self {
    Self::new()
  }
}

impl ExecutionMode {
  /// Create a new `InProcess` execution mode without shared memory.
  ///
  /// This is the default in-process mode that uses zero-copy execution
  /// with `Arc` for fan-out scenarios, but doesn't use shared memory.
  ///
  /// # Returns
  ///
  /// An `ExecutionMode::InProcess` variant with `use_shared_memory = false`
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::execution::ExecutionMode;
  ///
  /// let mode = ExecutionMode::new_in_process();
  /// ```
  #[must_use]
  pub fn new_in_process() -> Self {
    Self::InProcess {
      use_shared_memory: false,
    }
  }

  /// Create a new `InProcess` execution mode with shared memory enabled.
  ///
  /// This mode uses shared memory for ultra-high performance scenarios.
  /// Shared memory allows for even more efficient data sharing between nodes.
  ///
  /// # Returns
  ///
  /// An `ExecutionMode::InProcess` variant with `use_shared_memory = true`
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::execution::ExecutionMode;
  ///
  /// let mode = ExecutionMode::new_in_process_shared_memory();
  /// ```
  #[must_use]
  pub fn new_in_process_shared_memory() -> Self {
    Self::InProcess {
      use_shared_memory: true,
    }
  }

  /// Create a new `Distributed` execution mode with the given serializer.
  ///
  /// This mode serializes data for transmission between nodes, enabling
  /// distributed execution. Compression and batching are optional.
  ///
  /// # Arguments
  ///
  /// * `serializer` - The serializer to use for data serialization
  ///
  /// # Returns
  ///
  /// An `ExecutionMode::Distributed` variant with no compression or batching
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::execution::ExecutionMode;
  /// use streamweave::graph::serialization::Serializer;
  ///
  /// let serializer = super::serialization::JsonSerializer::default();
  /// let mode = ExecutionMode::new_distributed(serializer);
  /// ```
  #[must_use]
  pub fn new_distributed(serializer: super::serialization::JsonSerializer) -> Self {
    Self::Distributed {
      serializer,
      compression: None,
      batching: None,
    }
  }

  /// Create a new `Hybrid` execution mode with the given threshold and serializer.
  ///
  /// This mode starts in in-process mode and switches to distributed mode
  /// when the local threshold is exceeded.
  ///
  /// # Arguments
  ///
  /// * `local_threshold` - Threshold for switching from in-process to distributed mode
  /// * `serializer` - Serializer to use when in distributed mode
  ///
  /// # Returns
  ///
  /// An `ExecutionMode::Hybrid` variant with the specified threshold and serializer
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::execution::ExecutionMode;
  /// use streamweave::graph::serialization::Serializer;
  ///
  /// let serializer = super::serialization::JsonSerializer::default();
  /// let mode = ExecutionMode::new_hybrid(1000, serializer);
  /// ```
  #[must_use]
  pub fn new_hybrid(
    local_threshold: usize,
    serializer: super::serialization::JsonSerializer,
  ) -> Self {
    Self::Hybrid {
      local_threshold,
      serializer,
    }
  }
}

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
  /// Execution mode for this executor
  execution_mode: ExecutionMode,
  /// Shared memory channels for ultra-high performance mode
  /// Key: (node_name, port_name) for source nodes
  /// Value: Shared memory channel for that connection
  shared_memory_channels: HashMap<(String, String), SharedMemoryChannel>,
  /// Batching channels for distributed execution with batching enabled
  /// Key: (node_name, port_name) for source nodes
  /// Value: Batching channel wrapper for that connection
  batching_channels: HashMap<(String, String), Arc<BatchingChannel>>,
  /// Throughput monitor for hybrid execution mode
  /// Tracks items/second processed across the graph
  throughput_monitor: Option<Arc<ThroughputMonitor>>,
  /// Current actual execution mode (may differ from execution_mode during transitions)
  /// In hybrid mode, this tracks whether we're currently in-process or distributed
  current_execution_mode: Option<ExecutionMode>,
  /// Background task handle for throughput monitoring in hybrid mode
  throughput_monitoring_task: Option<tokio::task::JoinHandle<Result<(), ExecutionError>>>,
  /// Metrics for mode switching in hybrid mode
  mode_switch_metrics: Option<Arc<RwLock<ModeSwitchMetrics>>>,
}

impl GraphExecutor {
  /// Detects the optimal execution mode for the graph.
  ///
  /// This method analyzes the graph topology and configuration to determine
  /// the best execution mode. Factors considered include:
  /// - Whether all nodes are in the same process (in-process mode)
  /// - Whether network boundaries exist (distributed mode)
  /// - Graph complexity and size (hybrid mode)
  ///
  /// # Returns
  ///
  /// The recommended `ExecutionMode` for this graph.
  ///
  /// # Current Implementation
  ///
  /// Currently defaults to `InProcess` mode for zero-copy execution.
  /// Future enhancements will:
  /// - Detect network boundaries from node metadata
  /// - Analyze graph complexity to suggest hybrid mode
  /// - Consider node distribution across processes/machines
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  ///
  /// let graph = Graph::new();
  /// let executor = graph.executor();
  /// let recommended_mode = executor.detect_execution_mode();
  /// ```
  #[must_use]
  pub fn detect_execution_mode(&self) -> ExecutionMode {
    // For now, default to in-process mode for zero-copy execution
    // Future: Analyze graph topology to determine optimal mode
    // - Check if nodes have network metadata indicating distributed execution
    // - Analyze graph size and complexity
    // - Consider user preferences and configuration

    ExecutionMode::new_in_process()
  }

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
      arc_pool: None,
      execution_mode: ExecutionMode::new_in_process(), // Default to in-process for zero-copy
      shared_memory_channels: HashMap::new(),
      batching_channels: HashMap::new(),
      throughput_monitor: None,
      current_execution_mode: None,
      throughput_monitoring_task: None,
      mode_switch_metrics: None,
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
      execution_mode: ExecutionMode::new_in_process(), // Default to in-process for zero-copy
      shared_memory_channels: HashMap::new(),
      batching_channels: HashMap::new(),
      throughput_monitor: None,
      current_execution_mode: None,
      throughput_monitoring_task: None,
      mode_switch_metrics: None,
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
  /// use streamweave::graph::{Graph, GraphExecution};
  /// use streamweave::graph::ArcPool;
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
    // Route to appropriate execution method based on execution mode
    match &self.execution_mode {
      ExecutionMode::InProcess { use_shared_memory } => {
        self.execute_in_process(*use_shared_memory).await
      }
      ExecutionMode::Distributed { .. } => {
        // For distributed mode, extract configuration from execution_mode
        // Since we can't easily clone Box<dyn Serializer>, we'll use the stored one
        // This requires refactoring - for now, use execute_distributed directly
        // or extract from execution_mode (which we'll do in a helper)
        if let ExecutionMode::Distributed {
          serializer: _,
          compression: _,
          batching: _,
        } = &self.execution_mode
        {
          // We can't move serializer out, so we need a different approach
          // For now, nodes will use the serialize() function from serialization module
          // Future: pass serializer reference to nodes or make Serializer Clone
          self.execute_distributed_internal().await
        } else {
          unreachable!()
        }
      }
      ExecutionMode::Hybrid { .. } => {
        // Hybrid mode starts in-process, will be handled separately
        self.execute_in_process(false).await
      }
    }
  }

  /// Internal helper for distributed execution using stored execution_mode
  ///
  /// This method handles compression and batching configuration when present.
  /// Compression is applied to serialized data before transmission.
  /// Batching groups multiple items together for efficient transmission.
  async fn execute_distributed_internal(&mut self) -> Result<(), ExecutionError> {
    // Extract compression and batching config from execution_mode
    let (_compression, batching) = match &self.execution_mode {
      ExecutionMode::Distributed {
        compression,
        batching,
        ..
      } => (*compression, batching.clone()),
      _ => (None, None),
    };

    // Compression is now implemented in nodes (ProducerNode, TransformerNode)
    // Nodes check ExecutionMode and apply compression after serialization
    // Decompression is handled in StreamWrapper when receiving data

    // Batching is configured but not yet fully implemented
    // Future: Buffer items according to BatchConfig and send in batches
    if batching.is_some() {
      // TODO: Implement batching
      // Buffer items up to batch_size or batch_timeout_ms, then send as batch
    }
    if self.state == ExecutionState::Running {
      return Err(ExecutionError::ExecutionFailed(
        "Graph is already running".to_string(),
      ));
    }

    // Validate graph topology
    self.validate_topology()?;

    // Create channels for all connections
    self.create_channels()?;

    // Spawn tasks for each node with distributed mode
    for node_name in self.graph.node_names() {
      if let Some(node) = self.graph.get_node(node_name) {
        // Collect input channels for this node
        let mut input_channels = HashMap::new();
        let parents = self.graph.get_parents(node_name);
        for (parent_name, parent_port_name) in parents {
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
        let children = self.graph.get_children(node_name);
        for (child_name, child_port_name) in children {
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

        // Spawn execution task with distributed mode
        // Collect batching channels for this node if batching is enabled
        let batching_channels: Option<HashMap<String, Arc<BatchingChannel>>> = if matches!(
          self.execution_mode,
          ExecutionMode::Distributed {
            batching: Some(_),
            ..
          }
        ) {
          let mut batch_map = HashMap::new();
          for port_name in output_channels.keys() {
            if let Some(batching_channel) = self
              .batching_channels
              .get(&(node_name.to_string(), port_name.clone()))
            {
              batch_map.insert(port_name.clone(), Arc::clone(batching_channel));
            }
          }
          if !batch_map.is_empty() {
            Some(batch_map)
          } else {
            None
          }
        } else {
          None
        };

        // Clone arc_pool if available
        let arc_pool_clone = self.arc_pool.as_ref().map(|p| Arc::new(p.clone()));

        if let Some(handle) = node.spawn_execution_task(
          input_channels,
          output_channels,
          self.pause_signal.clone(),
          self.execution_mode.clone(),
          batching_channels,
          arc_pool_clone,
        ) {
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

  /// Executes the graph in distributed serialized mode.
  ///
  /// This method implements distributed execution by serializing data
  /// between nodes using the configured Serializer trait. Compression
  /// and batching options are respected when specified.
  ///
  /// # Arguments
  ///
  /// * `serializer` - The serializer to use for data serialization
  /// * `compression` - Optional compression algorithm for serialized data
  /// * `batching` - Optional batching configuration for network efficiency
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution started successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Distributed Execution Semantics
  ///
  /// - Data is serialized using the configured Serializer trait
  /// - Optional compression reduces network bandwidth
  /// - Optional batching improves network efficiency
  /// - Suitable for cross-process or cross-network execution
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  /// use streamweave::graph::execution::ExecutionMode;
  /// use streamweave::graph::serialization::Serializer;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// let serializer: Box<dyn Serializer> = /* your serializer */;
  /// executor.execute_distributed(serializer, None, None).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn execute_distributed(
    &mut self,
    serializer: super::serialization::JsonSerializer,
    compression: Option<CompressionAlgorithm>,
    batching: Option<BatchConfig>,
  ) -> Result<(), ExecutionError> {
    if self.state == ExecutionState::Running {
      return Err(ExecutionError::ExecutionFailed(
        "Graph is already running".to_string(),
      ));
    }

    // Set execution mode to Distributed
    self.execution_mode = ExecutionMode::Distributed {
      serializer,
      compression,
      batching,
    };

    // Use internal helper to execute
    self.execute_distributed_internal().await
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

    // Cleanup batching channels (flush remaining items)
    for channel in self.batching_channels.values() {
      let _ = channel.shutdown().await;
    }
    self.batching_channels.clear();

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

    // Cleanup batching channels
    for channel in self.batching_channels.values() {
      let _ = channel.shutdown().await;
    }
    self.batching_channels.clear();

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
  /// - Distributed mode: `ChannelItem::Bytes` contains serialized `Message<T>`
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
    let use_shared_memory = matches!(
      self.execution_mode,
      ExecutionMode::InProcess {
        use_shared_memory: true
      }
    );

    // Extract batching config from execution mode
    let batching_config = match &self.execution_mode {
      ExecutionMode::Distributed { batching, .. } => batching.clone(),
      _ => None,
    };

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

        // If batching is enabled, wrap the sender with BatchingChannel
        if let Some(ref batch_config) = batching_config {
          let batching_channel =
            Arc::new(BatchingChannel::new(sender.clone(), batch_config.clone()));
          self.batching_channels.insert(
            (conn.source.0.clone(), source_port_name.clone()),
            batching_channel,
          );

          // Still store the inner sender for backward compatibility
          // Nodes will check for batching and use BatchingChannel if available
          self
            .channel_senders
            .insert((conn.source.0.clone(), source_port_name.clone()), sender);
        } else {
          // Store sender for source node's output port
          self
            .channel_senders
            .insert((conn.source.0.clone(), source_port_name.clone()), sender);
        }

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
  /// use streamweave::graph::{Graph, GraphExecution};
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

    // Set execution mode to InProcess
    self.execution_mode = ExecutionMode::InProcess { use_shared_memory };

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
      if let Some(node) = self.graph.get_node(node_name) {
        // Collect input channels for this node
        let mut input_channels = HashMap::new();
        let parents = self.graph.get_parents(node_name);
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
        let children = self.graph.get_children(node_name);
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

        // Increment throughput monitor if available (for hybrid mode)
        let throughput_monitor = self.throughput_monitor.clone();
        let node_name_for_monitor = node_name.to_string();
        let node_name_for_insert = node_name_for_monitor.clone();

        // Spawn execution task
        // In in-process mode, nodes will use Arc<Message<T>> channels and direct stream passing
        // The node execution code checks ExecutionMode and uses appropriate channel types
        // All data is wrapped in Message<T> for end-to-end traceability
        // Clone arc_pool if available
        let arc_pool_clone = self.arc_pool.as_ref().map(|p| Arc::new(p.clone()));

        if let Some(handle) = node.spawn_execution_task(
          input_channels,
          output_channels,
          self.pause_signal.clone(),
          ExecutionMode::InProcess { use_shared_memory },
          None, // No batching in in-process mode
          arc_pool_clone,
        ) {
          // Wrap handle to track throughput
          if let Some(monitor) = throughput_monitor {
            let monitor_clone = Arc::clone(&monitor);
            let wrapped_handle = tokio::spawn(async move {
              match handle.await {
                Ok(Ok(())) => {
                  // Increment throughput when task completes (simplified - actual tracking
                  // should be per-item, which will be added in node execution)
                  monitor_clone.increment_item_count();
                  Ok(())
                }
                Ok(Err(e)) => Err(e),
                Err(e) => Err(ExecutionError::NodeExecutionFailed {
                  node: node_name_for_monitor,
                  reason: format!("Task join error: {}", e),
                  message_id: None,
                }),
              }
            });
            self
              .node_handles
              .insert(node_name_for_insert, wrapped_handle);
          } else {
            self.node_handles.insert(node_name.to_string(), handle);
          }
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

  /// Executes the graph in hybrid mode.
  ///
  /// Hybrid mode starts in in-process mode and switches to distributed mode
  /// when the local threshold is exceeded. This enables adaptive performance
  /// optimization based on load.
  ///
  /// # Arguments
  ///
  /// * `local_threshold` - Threshold for switching from in-process to distributed
  /// * `serializer` - Serializer to use when switching to distributed mode
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution started successfully, `Err(ExecutionError)` otherwise.
  ///
  /// # Hybrid Mode Semantics
  ///
  /// - Starts in in-process zero-copy mode
  /// - Monitors throughput and load
  /// - Switches to distributed mode when threshold exceeded
  /// - Provides adaptive performance optimization
  ///
  /// # Note
  ///
  /// Mode switching is currently a placeholder. Future implementation will:
  /// - Monitor item count or throughput
  /// - Dynamically switch execution modes
  /// - Handle state migration between modes
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::{Graph, GraphExecution};
  /// use streamweave::graph::execution::ExecutionMode;
  /// use streamweave::graph::serialization::Serializer;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let graph = Graph::new();
  /// let mut executor = graph.executor();
  ///
  /// let serializer: Box<dyn Serializer> = /* your serializer */;
  /// executor.execute_hybrid(1000, serializer).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn execute_hybrid(
    &mut self,
    local_threshold: usize,
    serializer: super::serialization::JsonSerializer,
  ) -> Result<(), ExecutionError> {
    if self.state == ExecutionState::Running {
      return Err(ExecutionError::ExecutionFailed(
        "Graph is already running".to_string(),
      ));
    }

    // Set execution mode to Hybrid
    self.execution_mode = ExecutionMode::Hybrid {
      local_threshold,
      serializer: serializer.clone(),
    };

    // Initialize throughput monitor for hybrid mode
    let throughput_monitor = Arc::new(ThroughputMonitor::new(Duration::from_secs(1)));
    self.throughput_monitor = Some(Arc::clone(&throughput_monitor));
    self.current_execution_mode = Some(ExecutionMode::InProcess {
      use_shared_memory: false,
    });

    // Initialize mode switch metrics
    let metrics = Arc::new(RwLock::new(ModeSwitchMetrics::new()));
    self.mode_switch_metrics = Some(Arc::clone(&metrics));

    // Start in in-process mode
    self.execute_in_process(false).await?;

    // Start background throughput monitoring task
    let monitor_task = self
      .start_throughput_monitoring(Arc::clone(&throughput_monitor), local_threshold)
      .await?;
    self.throughput_monitoring_task = Some(monitor_task);

    Ok(())
  }

  /// Start background throughput monitoring task for hybrid mode.
  ///
  /// This task periodically checks throughput and triggers mode switches
  /// when the threshold is exceeded.
  ///
  /// # Arguments
  ///
  /// * `monitor` - The throughput monitor to use
  /// * `local_threshold` - Threshold for switching to distributed mode (items/second)
  ///
  /// # Returns
  ///
  /// A join handle for the monitoring task.
  ///
  /// # Note
  ///
  /// The monitoring task will check throughput periodically and log when
  /// threshold is exceeded. The actual mode switch will be triggered by
  /// calling `trigger_mode_switch` method.
  async fn start_throughput_monitoring(
    &mut self,
    monitor: Arc<ThroughputMonitor>,
    local_threshold: usize,
  ) -> Result<tokio::task::JoinHandle<Result<(), ExecutionError>>, ExecutionError> {
    let pause_signal = Arc::clone(&self.pause_signal);

    let handle = tokio::spawn(async move {
      let monitoring_interval = Duration::from_millis(100); // Check every 100ms

      loop {
        tokio::time::sleep(monitoring_interval).await;

        // Check if we should stop (graph stopped or paused)
        if *pause_signal.read().await {
          // Check if this is a shutdown or just a pause
          // For now, exit on pause (can be refined later)
          break;
        }

        // Calculate current throughput
        let throughput = monitor.calculate_throughput().await;

        // Check if we should switch to distributed mode
        if throughput > local_threshold as f64 {
          // Signal that a mode switch is needed
          tracing::info!(
            throughput = throughput,
            threshold = local_threshold,
            "Throughput exceeded threshold, mode switch needed"
          );
          // Note: Actual mode switch will be handled by calling trigger_mode_switch
          // from the main executor. This task just monitors and logs.
        }
      }

      Ok(())
    });

    Ok(handle)
  }

  /// Trigger a mode switch from in-process to distributed mode.
  ///
  /// This method handles the graceful transition:
  /// 1. Pause all nodes
  /// 2. Drain in-process channels
  /// 3. Create distributed channels
  /// 4. Migrate state
  /// 5. Resume with distributed mode
  ///
  /// # Returns
  ///
  /// `Ok(())` if the switch completed successfully, `Err` otherwise.
  pub async fn trigger_mode_switch(&mut self) -> Result<(), ExecutionError> {
    // Check if we're in hybrid mode and currently in-process
    let (local_threshold, serializer) = match &self.execution_mode {
      ExecutionMode::Hybrid {
        local_threshold,
        serializer,
      } => (*local_threshold, serializer.clone()),
      _ => {
        return Err(ExecutionError::ExecutionFailed(
          "Mode switch can only be triggered in hybrid mode".to_string(),
        ));
      }
    };

    if !matches!(
      self.current_execution_mode,
      Some(ExecutionMode::InProcess { .. })
    ) {
      // Already in distributed mode or not initialized
      return Ok(());
    }

    // Get current throughput for metrics
    let current_throughput = if let Some(monitor) = &self.throughput_monitor {
      monitor.calculate_throughput().await
    } else {
      0.0
    };

    tracing::info!(
      throughput = current_throughput,
      threshold = local_threshold,
      "Starting mode switch from in-process to distributed (throughput {} > threshold {})",
      current_throughput,
      local_threshold
    );

    // Record mode switch in metrics
    if let Some(metrics) = &self.mode_switch_metrics {
      metrics
        .write()
        .await
        .record_switch(current_throughput, "InProcess", "Distributed");
    }

    // Step 1: Pause all nodes
    *self.pause_signal.write().await = true;

    // Step 2: Wait for nodes to finish current items (with timeout)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Step 3: Drain in-process channels and collect in-flight items with connection info
    let in_flight_items = self.drain_in_process_channels().await;

    // Step 4: Stop current node tasks
    for handle in self.node_handles.values() {
      handle.abort();
    }
    self.node_handles.clear();

    // Step 5: Create distributed channels
    self.channel_senders.clear();
    self.channel_receivers.clear();
    self.create_channels()?;

    // Step 6: Spawn nodes with distributed mode
    for node_name in self.graph.node_names() {
      if let Some(node) = self.graph.get_node(node_name) {
        // Collect input channels
        let mut input_channels = HashMap::new();
        let parents = self.graph.get_parents(node_name);
        for (parent_name, parent_port_name) in parents {
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

        // Collect output channels
        let mut output_channels = HashMap::new();
        let children = self.graph.get_children(node_name);
        for (child_name, child_port_name) in children {
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

        // Spawn with distributed mode
        let distributed_mode = ExecutionMode::Distributed {
          serializer: serializer.clone(),
          compression: None,
          batching: None,
        };

        // Clone arc_pool if available
        let arc_pool_clone = self.arc_pool.as_ref().map(|p| Arc::new(p.clone()));

        if let Some(handle) = node.spawn_execution_task(
          input_channels,
          output_channels,
          self.pause_signal.clone(),
          distributed_mode,
          None, // No batching initially
          arc_pool_clone,
        ) {
          self.node_handles.insert(node_name.to_string(), handle);
        }
      }
    }

    // Step 7: Send in-flight items to new distributed channels
    let in_flight_count = in_flight_items.len();
    self.send_in_flight_items(in_flight_items).await?;

    // Step 8: Update current execution mode
    self.current_execution_mode = Some(ExecutionMode::Distributed {
      serializer: serializer.clone(),
      compression: None,
      batching: None,
    });

    // Step 9: Resume processing
    *self.pause_signal.write().await = false;

    tracing::info!(
      in_flight_migrated = in_flight_count,
      "Mode switch completed: now in distributed mode (migrated {} in-flight items)",
      in_flight_count
    );

    // Log metrics summary
    if let Some(metrics) = &self.mode_switch_metrics {
      let metrics_guard = metrics.read().await;
      tracing::info!("Mode switch metrics: {}", metrics_guard.summary());
    }

    Ok(())
  }

  /// Get mode switch metrics (if available).
  ///
  /// # Returns
  ///
  /// `Some(&Arc<RwLock<ModeSwitchMetrics>>)` if metrics are being tracked, `None` otherwise.
  pub fn mode_switch_metrics(&self) -> Option<&Arc<RwLock<ModeSwitchMetrics>>> {
    self.mode_switch_metrics.as_ref()
  }

  /// Drain in-process channels and collect in-flight items with connection information.
  ///
  /// # Returns
  ///
  /// Vector of tuples containing (connection_info, item) for routing items correctly.
  async fn drain_in_process_channels(
    &mut self,
  ) -> Vec<((String, String), (String, String), ChannelItem)> {
    let mut in_flight_items = Vec::new();

    // Collect items from all receivers, tracking which connection they belong to
    // The key in channel_receivers is (target_node, target_port_name)
    // We need to find the corresponding source node and port from connections
    for (target_key, receiver) in &mut self.channel_receivers {
      // Find the source node and port for this receiver
      // ConnectionInfo now stores port names directly
      let source_info = self.graph.get_connections().iter().find_map(|conn| {
        // Match by target node name and port name
        if conn.target.0 == target_key.0 && conn.target.1 == target_key.1 {
          // ConnectionInfo stores port names directly
          Some((conn.source.0.clone(), conn.source.1.clone()))
        } else {
          None
        }
      });

      if let Some(source_info) = source_info {
        // Try to receive remaining items (non-blocking)
        while let Ok(item) = receiver.try_recv() {
          in_flight_items.push((source_info.clone(), target_key.clone(), item));
        }
      }
    }

    in_flight_items
  }

  /// Send in-flight items to distributed channels.
  ///
  /// This method routes items to the correct distributed channels based on
  /// their connection information. Items from in-process mode (Arc<Message<T>>) are
  /// serialized to Bytes (containing serialized Message<T>) before sending.
  ///
  /// # Arguments
  ///
  /// * `items` - Vector of (source_info, target_info, item) tuples
  ///
  /// # Returns
  ///
  /// `Ok(())` if all items were sent successfully, `Err` otherwise.
  #[allow(clippy::type_complexity)]
  async fn send_in_flight_items(
    &self,
    items: Vec<((String, String), (String, String), ChannelItem)>,
  ) -> Result<(), ExecutionError> {
    // Get serializer from execution mode
    let _serializer = match &self.execution_mode {
      ExecutionMode::Hybrid { serializer, .. } => serializer,
      _ => {
        return Err(ExecutionError::ExecutionFailed(
          "send_in_flight_items can only be called in hybrid mode".to_string(),
        ));
      }
    };

    // Route each item to the correct distributed channel
    for (source_info, _target_info, item) in items {
      // Find the channel sender for this connection
      let channel_key = (source_info.0.clone(), source_info.1.clone());
      let sender = self.channel_senders.get(&channel_key);

      if let Some(sender) = sender {
        // Convert item to Bytes if needed
        let bytes_item = match item {
          ChannelItem::Bytes(bytes) => {
            // Already serialized, send directly
            ChannelItem::Bytes(bytes)
          }
          ChannelItem::Arc(_arc) => {
            // Need to serialize Arc<Message<T>> to Bytes
            // Try to downcast to Arc<Message<T>> and serialize
            // This is a limitation - we need to know the payload type to serialize
            // In practice, nodes should track the type information
            // For now, log a warning and skip items we can't serialize
            // TODO: Implement proper Message<T> serialization during mode switch
            tracing::warn!(
              source_node = %source_info.0,
              source_port = %source_info.1,
              "Cannot serialize Arc<Message<T>> item during mode switch - item will be dropped. \
               This requires type information that is not available at runtime."
            );
            continue;
          }
          ChannelItem::SharedMemory(_) => {
            // Shared memory items need special handling
            tracing::warn!(
              source_node = %source_info.0,
              source_port = %source_info.1,
              "SharedMemory items not supported in mode switch - item will be dropped"
            );
            continue;
          }
        };

        // Send to distributed channel
        if let Err(e) = sender.send(bytes_item).await {
          return Err(ExecutionError::ChannelError {
            node: source_info.0,
            port: source_info.1,
            is_input: false,
            reason: format!("Failed to send in-flight item: {}", e),
            message_id: None,
          });
        }
      } else {
        tracing::warn!(
          source_node = %source_info.0,
          source_port = %source_info.1,
          target_node = %_target_info.0,
          target_port = %_target_info.1,
          "No channel sender found for connection - item will be dropped"
        );
      }
    }

    Ok(())
  }

  /// Get the throughput monitor (if available).
  ///
  /// # Returns
  ///
  /// `Some(&Arc<ThroughputMonitor>)` if monitoring is active, `None` otherwise.
  pub fn throughput_monitor(&self) -> Option<&Arc<ThroughputMonitor>> {
    self.throughput_monitor.as_ref()
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
  /// send as `ChannelItem::Bytes` (serialized `Message<T>`) or `ChannelItem::Arc`
  /// (`Arc<Message<T>>`) based on `ExecutionMode`.
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
  /// Nodes should extract `ChannelItem::Bytes` (deserialize to `Message<T>`) or
  /// `ChannelItem::Arc` (downcast to `Arc<Message<T>>`) based on `ExecutionMode`,
  /// then unwrap the `Message<T>` to get the payload.
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
    let mut executor = GraphExecutor::new(self);
    // Set execution mode from graph
    executor.execution_mode = executor.graph.execution_mode().clone();
    executor
  }
}
