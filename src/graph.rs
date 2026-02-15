//! # Graph - Pure Stream Implementation
//!
//! This module defines the `Graph` struct for managing graph structures and executing
//! async node graphs. Graphs contain nodes and edges, and provide methods for both
//! structure management (sync) and execution (async).
//!
//! ## Stream-Based Execution
//!
//! The graph execution engine works purely with streams:
//!
//! 1. Collects input streams for each node from connected upstream nodes
//! 2. Calls `node.execute(inputs)` which returns output streams
//! 3. Connects output streams to downstream nodes' input streams
//! 4. Drives all streams to completion
//!
//! Channels are used internally for backpressure, but are never exposed to nodes.
//! Nodes only see and work with streams.
//!
//! ## Graph as Node (Nested Graphs)
//!
//! `Graph` implements the `Node` trait, allowing graphs to be used as nodes within
//! other graphs. This enables hierarchical composition and reusable subgraphs.
//!
//! ### Port Mapping
//!
//! When a graph is used as a node, you must explicitly map internal node ports to
//! external ports using `expose_input_port()` and `expose_output_port()`:
//!
//! - **Input ports**: `"configuration"` and `"input"` (fixed external names)
//! - **Output ports**: `"output"` and `"error"` (fixed external names)
//!
//! ### Pull-Based Execution
//!
//! Graphs use a pull-based execution model:
//!
//! 1. When a graph's output port is consumed, it signals readiness backward
//! 2. This propagates through internal nodes to the graph's input ports
//! 3. Data flows only when downstream nodes are ready to consume
//!
//! ### Lifecycle Control
//!
//! Graphs support full lifecycle control:
//!
//! - `start()` - Begin execution (sets state to running)
//! - `pause()` - Pause execution (maintains state, stops processing new data)
//! - `resume()` - Resume execution after pause
//! - `stop()` - Stop execution and clear all state (discards in-flight data)
//!
//! ### Example: Nested Graphs
//!
//! ```rust,no_run
//! use streamweave;
//! use streamweave::node::Node;
//! use streamweave::edge::Edge;
//!
//! // Create a subgraph
//! let mut subgraph = Graph::new("subgraph".to_string());
//! // ... add nodes and edges to subgraph ...
//!
//! // Expose internal ports as external ports
//! subgraph.expose_input_port("internal_source", "in", "input")?;
//! subgraph.expose_output_port("internal_sink", "out", "output")?;
//!
//! // Use subgraph as a node in parent graph
//! let mut parent = Graph::new("parent".to_string());
//! let subgraph_node: Box<dyn Node> = Box::new(subgraph);
//! parent.add_node("subgraph".to_string(), subgraph_node)?;
//!
//! // Connect to subgraph's external ports
//! parent.add_edge(Edge {
//!     source_node: "source".to_string(),
//!     source_port: "out".to_string(),
//!     target_node: "subgraph".to_string(),
//!     target_port: "input".to_string(),
//! })?;
//! ```

use crate::incremental::{plan_recompute as incremental_plan_recompute, RecomputePlan, RecomputeRequest};
use crate::checkpoint::{
    CheckpointDone, CheckpointId, CheckpointMetadata, CheckpointStorage,
    DistributedCheckpointStorage,
};
use crate::edge::Edge;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::partitioning::PartitionKey;
use crate::supervision::{FailureAction, FailureReport, SupervisionPolicy};
use crate::time::{CompletedFrontier, LogicalTime, ProgressHandle, Timestamped};
use async_trait::async_trait;
use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;

/// Default channel capacity for dataflow edges (backpressure).
const DATAFLOW_CHANNEL_CAPACITY: usize = 64;

/// Shard identity for cluster-sharded execution.
///
/// When running N graph instances (e.g. one per Kafka partition), each instance
/// gets a `ShardConfig` so it knows which partition of the key space it owns.
/// Keys are assigned by `hash(key) % total_shards == shard_id`.
///
/// See [cluster-sharding.md](../docs/cluster-sharding.md).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShardConfig {
    /// This instance's shard index (0..total_shards-1).
    pub shard_id: u32,
    /// Total number of shards; keys are partitioned by hash % total_shards.
    pub total_shards: u32,
}

impl ShardConfig {
    /// Creates a new shard config.
    pub fn new(shard_id: u32, total_shards: u32) -> Self {
        Self { shard_id, total_shards }
    }

    /// Returns true if this shard owns the given partition key.
    ///
    /// Uses `hash(key) % total_shards == shard_id`. The driver should route
    /// records so that each instance only receives keys it owns; this method
    /// allows nodes to reject or filter unexpected keys.
    pub fn owns_key(&self, key: &str) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let h = hasher.finish();
        (h % self.total_shards as u64) == self.shard_id as u64
    }
}

/// Execution mode for the graph.
///
/// - **Concurrent**: Default. Each node runs in its own task; ordering between nodes is not guaranteed.
/// - **Deterministic**: Nodes are started in topological order; improves reproducibility for testing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum ExecutionMode {
    /// Concurrent tasks per node; order not guaranteed.
    #[default]
    Concurrent,
    /// Single-task-friendly: nodes started in topological order for reproducibility.
    Deterministic,
}

/// Progress frontier configuration for run_dataflow.
#[derive(Clone)]
enum ProgressFrontierConfig {
    /// Single shared frontier; all sinks advance the same frontier.
    Single(Arc<CompletedFrontier>),
    /// Per-sink frontiers; graph-level progress = min over all sink frontiers.
    PerSink(HashMap<(String, String), Arc<CompletedFrontier>>),
}

/// Message type for dataflow edges. When progress tracking is enabled, items carry
/// a logical time so the completed frontier can be advanced when items reach sinks.
#[derive(Clone)]
enum EdgeMessage {
    /// No progress tracking; payload only.
    PayloadOnly(Arc<dyn Any + Send + Sync>),
    /// Progress tracking enabled; payload and its logical time.
    #[allow(dead_code)] // time is carried on the channel; sender uses time_opt for frontier
    WithTime(Arc<dyn Any + Send + Sync>, LogicalTime),
}

/// Per-round feedback override for cyclic execution.
/// For each feedback edge, the target receives from `inject_rx` and the source's output
/// is captured via `capture_tx`. This breaks the cycle for round-based execution.
struct RoundFeedbackConfig {
  inject_and_capture: Vec<(
    Edge,
    tokio::sync::mpsc::Receiver<EdgeMessage>,
    tokio::sync::mpsc::Sender<EdgeMessage>,
  )>,
}

/// Error type for graph execution operations.
pub type GraphExecutionError = Box<dyn std::error::Error + Send + Sync>;

/// Handle for a task: `Some(node_id)` for node tasks, `None` for bridge tasks.
type TaskHandle = (Option<String>, JoinHandle<Result<(), GraphExecutionError>>);
/// Type alias for execution handles to reduce type complexity
type ExecutionHandleVec = Arc<Mutex<Vec<TaskHandle>>>;

/// Type alias for the map of nodes restored after dataflow run (reduces type complexity).
type NodesRestoredMap = Arc<Mutex<HashMap<String, Box<dyn Node>>>>;

/// A graph containing nodes and edges.
///
/// Graphs represent the structure of a data processing pipeline, with nodes
/// representing processing components and edges representing data flow between them.
///
/// # Graph Structure
///
/// A graph consists of:
///
/// - **Nodes**: Processing components that implement the `Node` trait
/// - **Edges**: Connections between node ports (stream connections)
/// - **Port Mappings**: For graphs used as nodes, mappings from internal to external ports
///
/// # Structure Management (Synchronous)
///
/// The graph provides synchronous methods for managing its structure:
///
/// - Adding and removing nodes
/// - Adding and removing edges
/// - Querying nodes and edges
/// - Exposing internal ports as external ports (for nested graphs)
///
/// These operations are synchronous because they only modify data structures.
///
/// # Execution (Asynchronous)
///
/// The graph provides asynchronous methods for executing the graph:
///
/// - `execute()` - Starts graph execution by connecting streams between nodes
/// - `start()` - Sets execution state to running (for pull-based execution)
/// - `pause()` - Pauses execution (maintains state)
/// - `resume()` - Resumes execution after pause
/// - `stop()` - Stops execution and clears all state
/// - `wait_for_completion()` - Waits for all nodes to complete execution
///
/// Execution is asynchronous and uses pure stream composition - no channels exposed.
///
/// # Graph as Node
///
/// `Graph` implements the `Node` trait, allowing graphs to be nested within other graphs.
/// When used as a node, a graph has fixed external ports:
///
/// - **Input ports**: `"configuration"`, `"input"`
/// - **Output ports**: `"output"`, `"error"`
///
/// Use `expose_input_port()` and `expose_output_port()` to map internal node ports
/// to these external ports.
///
/// # Example: Basic Graph
///
/// ```rust,no_run
/// use streamweave::graph::Graph;
/// use streamweave::node::Node;
/// use streamweave::edge::Edge;
/// use streamweave::nodes::variable_node::VariableNode;
///
/// let mut graph = Graph::new("my_graph".to_string());
/// graph.add_node("source".to_string(), Box::new(VariableNode::new("source".to_string()))).unwrap();
/// graph.add_node("sink".to_string(), Box::new(VariableNode::new("sink".to_string()))).unwrap();
/// graph.add_edge(Edge {
///     source_node: "source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "sink".to_string(),
///     target_port: "value".to_string(),
/// }).unwrap();
/// // Then: graph.execute().await?; graph.wait_for_completion().await?;
/// ```
///
/// # Example: Nested Graph (Subgraph)
///
/// ```rust,no_run
/// use streamweave::graph::Graph;
/// use streamweave::node::Node;
/// use streamweave::edge::Edge;
/// use streamweave::nodes::variable_node::VariableNode;
///
/// let mut subgraph = Graph::new("subgraph".to_string());
/// subgraph.add_node("internal_source".to_string(), Box::new(VariableNode::new("internal_source".to_string()))).unwrap();
/// subgraph.add_node("internal_transform".to_string(), Box::new(VariableNode::new("internal_transform".to_string()))).unwrap();
/// subgraph.add_edge(Edge {
///     source_node: "internal_source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "internal_transform".to_string(),
///     target_port: "value".to_string(),
/// }).unwrap();
///
/// subgraph.expose_input_port("internal_source", "value", "input").unwrap();
/// subgraph.expose_output_port("internal_transform", "out", "output").unwrap();
/// let mut parent = Graph::new("parent".to_string());
/// parent.add_node("source".to_string(), Box::new(VariableNode::new("source".to_string()))).unwrap();
/// let subgraph_node: Box<dyn Node> = Box::new(subgraph);
/// parent.add_node("subgraph".to_string(), subgraph_node).unwrap();
/// parent.add_edge(Edge {
///     source_node: "source".to_string(),
///     source_port: "out".to_string(),
///     target_node: "subgraph".to_string(),
///     target_port: "input".to_string(),
/// }).unwrap();
/// ```
/// Port mapping from external port name to internal (node, port).
#[derive(Clone, Debug)]
struct PortMapping {
  /// Internal node name
  node: String,
  /// Internal port name
  port: String,
}

/// A graph containing nodes and edges.
///
/// Graphs represent the structure of a data processing pipeline, with nodes
/// representing processing components and edges representing data flow between them.
///
/// See the module-level documentation for detailed information about graph execution,
/// nested graphs, lifecycle control, and usage examples.
pub struct Graph {
  /// The name of the graph.
  name: String,
  /// Map of node names to node instances. Wrapped for interior mutability so execute_internal can take nodes with &self.
  nodes: Arc<StdMutex<HashMap<String, Box<dyn Node>>>>,
  /// List of edges connecting nodes.
  edges: Vec<Edge>,
  /// Execution handles for spawned node tasks (used for wait_for_completion)
  execution_handles: ExecutionHandleVec,
  /// Stop signal for graceful shutdown
  stop_signal: Arc<tokio::sync::Notify>,
  /// Pause signal for pausing execution
  pause_signal: Arc<tokio::sync::Notify>,
  /// Execution state: 0 = stopped, 1 = running, 2 = paused
  execution_state: Arc<AtomicU8>,
  /// Mapping of external input ports to internal nodes/ports
  /// Key: external port name (e.g., "input") -> (internal_node, internal_port)
  input_port_mapping: HashMap<String, PortMapping>,
  /// Mapping of external output ports to internal nodes/ports
  /// Key: external port name (e.g., "output") -> (internal_node, internal_port)
  output_port_mapping: HashMap<String, PortMapping>,
  /// Connected input channels for external data (payload only; time from counter when progress enabled)
  /// Key: external port name -> receiver for input data
  connected_input_channels:
    HashMap<String, Option<tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>>>,
  /// Timestamped input channels (user-provided event time; used instead of counter when progress enabled)
  /// Key: external port name -> receiver for timestamped items
  connected_timestamped_input_channels:
    HashMap<String, Option<tokio::sync::mpsc::Receiver<Timestamped<Arc<dyn Any + Send + Sync>>>>>,
  /// Connected output channels for external data
  /// Key: external port name -> sender for output data
  connected_output_channels: HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
  /// Cached input port names for Node trait
  input_port_names: Vec<String>,
  /// Cached output port names for Node trait
  output_port_names: Vec<String>,
  /// After dataflow execution, nodes are returned here so they can be restored in wait_for_completion.
  nodes_restored_after_run: Option<NodesRestoredMap>,
  /// Execution mode: Concurrent (default) or Deterministic.
  execution_mode: ExecutionMode,
  /// Shard identity when running N instances; None = single instance.
  shard_config: Option<ShardConfig>,
  /// Position restored from checkpoint; used to initialize time counter when executing with progress.
  restored_position: Option<LogicalTime>,
  /// Sequence number for checkpoint ids.
  checkpoint_sequence: std::sync::atomic::AtomicU64,
  /// When set, node task failures are sent here for the supervisor.
  failure_tx: Option<tokio::sync::mpsc::Sender<FailureReport>>,
  /// True when execute() has started and graph is running (until stop or wait_for_completion).
  running: Arc<std::sync::atomic::AtomicBool>,
  /// Per-node supervision policy; falls back to default when not set.
  node_supervision_policies: HashMap<String, SupervisionPolicy>,
  /// Default policy for nodes without an explicit policy.
  default_supervision_policy: SupervisionPolicy,
  /// Supervision group: node_id -> group_id. Nodes in the same group share restart count for RestartGroup.
  node_supervision_groups: HashMap<String, String>,
}

impl Graph {
  /// Creates a new empty graph with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name for the graph
  ///
  /// # Returns
  ///
  /// A new `Graph` instance with no nodes or edges.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::Graph;
  ///
  /// let graph = Graph::new("my_graph".to_string());
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      name,
      nodes: Arc::new(StdMutex::new(HashMap::new())),
      edges: Vec::new(),
      execution_handles: Arc::new(Mutex::new(Vec::new())),
      stop_signal: Arc::new(tokio::sync::Notify::new()),
      pause_signal: Arc::new(tokio::sync::Notify::new()),
      execution_state: Arc::new(AtomicU8::new(0)), // 0 = stopped
      input_port_mapping: HashMap::new(),
      output_port_mapping: HashMap::new(),
      connected_input_channels: HashMap::new(),
      connected_timestamped_input_channels: HashMap::new(),
      connected_output_channels: HashMap::new(),
      input_port_names: Vec::new(),
      output_port_names: Vec::new(),
      nodes_restored_after_run: None,
      execution_mode: ExecutionMode::default(),
      shard_config: None,
      restored_position: None,
      checkpoint_sequence: std::sync::atomic::AtomicU64::new(0),
      failure_tx: None,
      running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
      node_supervision_policies: HashMap::new(),
      default_supervision_policy: SupervisionPolicy::default(),
      node_supervision_groups: HashMap::new(),
    }
  }

  /// Assigns a node to a supervision group.
  ///
  /// When the policy is [`FailureAction::RestartGroup`], all nodes in the same group share
  /// a restart count. If any node in the group fails, the count increments for the whole group.
  /// Nodes without a group are treated as their own group (restart count per node).
  ///
  /// See [actor-supervision-trees.md](../docs/actor-supervision-trees.md).
  pub fn set_supervision_group(&mut self, node_id: &str, group_id: &str) {
    self
      .node_supervision_groups
      .insert(node_id.to_string(), group_id.to_string());
  }

  /// Returns the supervision group id for a node, if set.
  pub fn supervision_group(&self, node_id: &str) -> Option<&str> {
    self.node_supervision_groups.get(node_id).map(|s| s.as_str())
  }

  /// Sets the supervision policy for a node.
  ///
  /// When a node fails (panic or `Err` from `execute`), the supervisor applies
  /// this policy. See [actor-supervision-trees.md](../docs/actor-supervision-trees.md).
  pub fn set_node_supervision_policy(&mut self, node_id: &str, policy: SupervisionPolicy) {
    self.node_supervision_policies.insert(node_id.to_string(), policy);
  }

  /// Sets the default supervision policy for nodes without an explicit policy.
  pub fn set_default_supervision_policy(&mut self, policy: SupervisionPolicy) {
    self.default_supervision_policy = policy;
  }

  /// Enables failure reporting: node task failures (Err or panic) are sent to the returned receiver.
  ///
  /// The supervisor (or caller) should receive from the returned channel and apply the
  /// configured policy (restart, stop, escalate). See [actor-supervision-trees.md](../docs/actor-supervision-trees.md).
  pub fn enable_failure_reporting(&mut self) -> tokio::sync::mpsc::Receiver<FailureReport> {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    self.failure_tx = Some(tx);
    rx
  }

  /// Sets shard identity for cluster-sharded execution.
  ///
  /// When running N graph instances (e.g. one per Kafka partition), set this so
  /// nodes can behave differently per shard (e.g. state path). Keys are assigned
  /// by `hash(key) % total_shards == shard_id`.
  pub fn set_shard_config(&mut self, shard_id: u32, total_shards: u32) {
    self.shard_config = Some(ShardConfig::new(shard_id, total_shards));
  }

  /// Returns this instance's shard id, or None if not sharded.
  pub fn shard_id(&self) -> Option<u32> {
    self.shard_config.as_ref().map(|c| c.shard_id)
  }

  /// Returns the total number of shards, or None if not sharded.
  pub fn total_shards(&self) -> Option<u32> {
    self.shard_config.as_ref().map(|c| c.total_shards)
  }

  /// Returns shard config if this graph is running as a sharded instance.
  pub fn shard_config(&self) -> Option<ShardConfig> {
    self.shard_config
  }

  /// Triggers a checkpoint, saving state from all nodes to storage.
  ///
  /// Call when the graph is quiescent (before [`execute`](Self::execute) or after
  /// [`wait_for_completion`](Self::wait_for_completion)). Collects snapshots from
  /// nodes that implement [`Node::snapshot_state`](crate::node::Node::snapshot_state)
  /// and saves to storage.
  ///
  /// Returns the checkpoint id, or an error if no nodes are available (e.g. graph
  /// is executing) or if save fails.
  pub fn trigger_checkpoint(
    &self,
    storage: &dyn CheckpointStorage,
  ) -> Result<CheckpointId, GraphExecutionError> {
    let nodes = self.nodes.lock().unwrap();
    if nodes.is_empty() {
      return Err("Cannot checkpoint: no nodes (graph may be executing)".into());
    }

    let id = CheckpointId::new(
      self
        .checkpoint_sequence
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
    );

    let mut snapshots = HashMap::new();
    for (node_id, node) in nodes.iter() {
      match node.snapshot_state() {
        Ok(data) if !data.is_empty() => {
          snapshots.insert(node_id.clone(), data);
        }
        Ok(_) => {}
        Err(e) => return Err(format!("Node '{}' snapshot failed: {}", node_id, e).into()),
      }
    }

    let metadata = CheckpointMetadata {
      id,
      position: self.restored_position,
    };
    storage
      .save(&metadata, &snapshots)
      .map_err(|e| format!("Checkpoint save failed: {}", e))?;
    Ok(id)
  }

  /// Triggers a checkpoint for coordinated distributed execution.
  ///
  /// Snapshots all stateful nodes and writes to shared storage under
  /// `<base>/<id>/shard_<shard_id>/`. Returns a [`CheckpointDone`] that the
  /// caller must report to the coordinator via
  /// [`CheckpointCoordinator::report_done`](crate::checkpoint::CheckpointCoordinator::report_done).
  ///
  /// Use this when the graph runs as a shard in a coordinated checkpoint
  /// (see [distributed-checkpointing.md](../docs/distributed-checkpointing.md) §6).
  pub fn trigger_checkpoint_for_coordination(
    &self,
    storage: &dyn DistributedCheckpointStorage,
    request: &crate::checkpoint::CheckpointRequest,
    shard_id: u32,
  ) -> Result<CheckpointDone, GraphExecutionError> {
    let nodes = self.nodes.lock().unwrap();
    if nodes.is_empty() {
      return Err("Cannot checkpoint: no nodes (graph may be executing)".into());
    }

    let mut snapshots = HashMap::new();
    for (node_id, node) in nodes.iter() {
      match node.snapshot_state() {
        Ok(data) if !data.is_empty() => {
          snapshots.insert(node_id.clone(), data);
        }
        Ok(_) => {}
        Err(_e) => {
          return Ok(CheckpointDone {
            checkpoint_id: request.checkpoint_id,
            shard_id,
            success: false,
          });
        }
      }
    }

    let metadata = CheckpointMetadata {
      id: request.checkpoint_id,
      position: self.restored_position,
    };

    match storage.save_shard(request.checkpoint_id, shard_id, &metadata, &snapshots) {
      Ok(()) => Ok(CheckpointDone {
        checkpoint_id: request.checkpoint_id,
        shard_id,
        success: true,
      }),
      Err(_) => Ok(CheckpointDone {
        checkpoint_id: request.checkpoint_id,
        shard_id,
        success: false,
      }),
    }
  }

  /// Restores graph state from a checkpoint.
  ///
  /// Loads the checkpoint from storage, restores state into nodes that support
  /// [`Node::restore_state`](crate::node::Node::restore_state), and records the
  /// checkpoint position. When execution starts with progress tracking, the time
  /// counter is initialized from this position so replay resumes correctly.
  ///
  /// Call this before [`execute`](Self::execute) or [`execute_with_progress`](Self::execute_with_progress)
  /// to resume from a previous checkpoint (e.g. after a process restart).
  ///
  /// # Errors
  ///
  /// Returns an error if the checkpoint is not found or if any node fails to restore.
  pub fn restore_from_checkpoint(
    &mut self,
    storage: &dyn CheckpointStorage,
    id: CheckpointId,
  ) -> Result<(), GraphExecutionError> {
    let (metadata, snapshots) = storage
      .load(id)
      .map_err(|e| format!("Checkpoint load failed: {}", e))?;

    let mut nodes = self.nodes.lock().unwrap();
    for (node_id, data) in &snapshots {
      if let Some(node) = nodes.get_mut(node_id) {
        node
          .restore_state(data)
          .map_err(|e| format!("Node '{}' restore failed: {}", node_id, e))?;
      }
    }

    self.restored_position = metadata.position;
    Ok(())
  }

  /// Restores graph state from a coordinated distributed checkpoint.
  ///
  /// Loads this shard's snapshot from shared storage. Call after the coordinator
  /// has committed the checkpoint (e.g. on worker recovery).
  pub fn restore_from_distributed_checkpoint(
    &mut self,
    storage: &dyn DistributedCheckpointStorage,
    id: CheckpointId,
    shard_id: u32,
  ) -> Result<(), GraphExecutionError> {
    let (metadata, snapshots) = storage
      .load_shard(id, shard_id)
      .map_err(|e| format!("Distributed checkpoint load failed: {}", e))?;

    let mut nodes = self.nodes.lock().unwrap();
    for (node_id, data) in &snapshots {
      if let Some(node) = nodes.get_mut(node_id) {
        node
          .restore_state(data)
          .map_err(|e| format!("Node '{}' restore failed: {}", node_id, e))?;
      }
    }

    self.restored_position = metadata.position;
    Ok(())
  }

  /// Exports state for the given partition keys from a node (for state migration during rebalance).
  ///
  /// When this shard loses keys to another shard, call this to extract state for those keys.
  /// The returned bytes can be sent to the shard that gains the keys, which calls
  /// [`import_state_for_keys`](Self::import_state_for_keys).
  ///
  /// Nodes that do not support key-scoped state return empty bytes.
  pub fn export_state_for_keys(
    &self,
    node_id: &str,
    keys: &[PartitionKey],
  ) -> Result<Vec<u8>, GraphExecutionError> {
    let nodes = self.nodes.lock().unwrap();
    let node = nodes
      .get(node_id)
      .ok_or_else(|| format!("Node '{}' not found", node_id))?;
    node
      .export_state_for_keys(keys)
      .map_err(|e| format!("Node '{}' export failed: {}", node_id, e).into())
  }

  /// Imports state for keys into a node (for state migration during rebalance).
  ///
  /// When this shard gains keys from another shard, call with the bytes from
  /// [`export_state_for_keys`](Self::export_state_for_keys) on the source shard.
  pub fn import_state_for_keys(
    &self,
    node_id: &str,
    data: &[u8],
  ) -> Result<(), GraphExecutionError> {
    let mut nodes = self.nodes.lock().unwrap();
    let node = nodes
      .get_mut(node_id)
      .ok_or_else(|| format!("Node '{}' not found", node_id))?;
    node
      .import_state_for_keys(data)
      .map_err(|e| format!("Node '{}' import failed: {}", node_id, e).into())
  }

  /// Returns the position restored from the last checkpoint, if any.
  pub fn restored_position(&self) -> Option<LogicalTime> {
    self.restored_position
  }

  /// Sets the execution mode.
  ///
  /// Use [`ExecutionMode::Deterministic`] for reproducible runs (e.g. tests).
  pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
    self.execution_mode = mode;
  }

  /// Returns the current execution mode.
  pub fn execution_mode(&self) -> ExecutionMode {
    self.execution_mode
  }

  /// Exposes an internal node's input port as an external input port.
  ///
  /// This allows external streams to flow into the graph through specific internal nodes.
  ///
  /// # Arguments
  ///
  /// * `internal_node` - The name of the internal node
  /// * `internal_port` - The name of the input port on the internal node
  /// * `external_name` - The name of the external port (must be "configuration" or "input")
  ///
  /// # Returns
  ///
  /// `Ok(())` if the port was exposed successfully, or an error if:
  /// - The internal node doesn't exist
  /// - The internal port doesn't exist on the node
  /// - The external port name is invalid (must be "configuration" or "input")
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("source".to_string(), Box::new(VariableNode::new("source".to_string()))).unwrap();
  /// graph.expose_input_port("source", "value", "input").unwrap();
  /// ```
  pub fn expose_input_port(
    &mut self,
    internal_node: &str,
    internal_port: &str,
    external_name: &str,
  ) -> Result<(), String> {
    // Validate internal node exists
    let guard = self.nodes.lock().unwrap();
    let node = guard
      .get(internal_node)
      .ok_or_else(|| format!("Internal node '{}' does not exist", internal_node))?;

    // Validate internal port exists
    if !node.has_input_port(internal_port) {
      return Err(format!(
        "Internal node '{}' does not have input port '{}'",
        internal_node, internal_port
      ));
    }

    // Add mapping
    self.input_port_mapping.insert(
      external_name.to_string(),
      PortMapping {
        node: internal_node.to_string(),
        port: internal_port.to_string(),
      },
    );

    // Add external port name to the list if not already present
    if !self.input_port_names.contains(&external_name.to_string()) {
      self.input_port_names.push(external_name.to_string());
    }

    Ok(())
  }

  /// Exposes an internal node's output port as an external output port.
  ///
  /// This allows internal streams to flow out of the graph through specific internal nodes.
  ///
  /// # Arguments
  ///
  /// * `internal_node` - The name of the internal node
  /// * `internal_port` - The name of the output port on the internal node
  /// * `external_name` - The name of the external port (must be "output" or "error")
  ///
  /// # Returns
  ///
  /// `Ok(())` if the port was exposed successfully, or an error if:
  /// - The internal node doesn't exist
  /// - The internal port doesn't exist on the node
  /// - The external port name is invalid (must be "output" or "error")
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("sink".to_string(), Box::new(VariableNode::new("sink".to_string()))).unwrap();
  /// graph.expose_output_port("sink", "out", "output").unwrap();
  /// ```
  pub fn expose_output_port(
    &mut self,
    internal_node: &str,
    internal_port: &str,
    external_name: &str,
  ) -> Result<(), String> {
    // Validate internal node exists
    let guard = self.nodes.lock().unwrap();
    let node = guard
      .get(internal_node)
      .ok_or_else(|| format!("Internal node '{}' does not exist", internal_node))?;

    // Validate internal port exists
    if !node.has_output_port(internal_port) {
      return Err(format!(
        "Internal node '{}' does not have output port '{}'",
        internal_node, internal_port
      ));
    }

    // Add mapping
    self.output_port_mapping.insert(
      external_name.to_string(),
      PortMapping {
        node: internal_node.to_string(),
        port: internal_port.to_string(),
      },
    );

    // Add external port name to the list if not already present
    if !self.output_port_names.contains(&external_name.to_string()) {
      self.output_port_names.push(external_name.to_string());
    }

    Ok(())
  }

  /// Connects an input channel to an exposed input port.
  ///
  /// This allows external data to be sent to the graph through the specified port.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed input port
  /// * `receiver` - The channel receiver for input data
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was successful, or an error if the port is not exposed.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// use tokio::sync::mpsc;
  /// use std::sync::Arc;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("n".to_string(), Box::new(VariableNode::new("n".to_string()))).unwrap();
  /// graph.expose_input_port("n", "value", "configuration").unwrap();
  /// let (tx, rx) = mpsc::channel(10);
  /// graph.connect_input_channel("configuration", rx).unwrap();
  /// ```
  pub fn connect_input_channel(
    &mut self,
    external_port: &str,
    receiver: tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
  ) -> Result<(), String> {
    if !self.input_port_mapping.contains_key(external_port) {
      return Err(format!(
        "External input port '{}' is not exposed",
        external_port
      ));
    }
    self
      .connected_input_channels
      .insert(external_port.to_string(), Some(receiver));
    Ok(())
  }

  /// Connects a timestamped input channel to an exposed input port.
  ///
  /// Use this when the external source provides event time (e.g. Kafka timestamp,
  /// `created_at` from payload). With [`execute_with_progress`](Self::execute_with_progress),
  /// the user-provided time is used as the logical time instead of an internal counter.
  /// A port must have either [`connect_input_channel`](Self::connect_input_channel) or
  /// `connect_timestamped_input_channel`, not both.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed input port
  /// * `receiver` - The channel receiver for timestamped items (payload + logical time)
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was successful.
  pub fn connect_timestamped_input_channel(
    &mut self,
    external_port: &str,
    receiver: tokio::sync::mpsc::Receiver<Timestamped<Arc<dyn Any + Send + Sync>>>,
  ) -> Result<(), String> {
    if !self.input_port_mapping.contains_key(external_port) {
      return Err(format!(
        "External input port '{}' is not exposed",
        external_port
      ));
    }
    self
      .connected_timestamped_input_channels
      .insert(external_port.to_string(), Some(receiver));
    Ok(())
  }

  /// Connects an output channel to an exposed output port.
  ///
  /// This allows graph output to be sent to external consumers through the specified port.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed output port
  /// * `sender` - The channel sender for output data
  ///
  /// # Returns
  ///
  /// `Ok(())` if the connection was successful, or an error if the port is not exposed.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// use tokio::sync::mpsc;
  /// use std::sync::Arc;
  /// let mut graph = Graph::new("g".to_string());
  /// graph.add_node("n".to_string(), Box::new(VariableNode::new("n".to_string()))).unwrap();
  /// graph.expose_output_port("n", "out", "output").unwrap();
  /// let (tx, rx) = mpsc::channel(10);
  /// graph.connect_output_channel("output", tx).unwrap();
  /// ```
  pub fn connect_output_channel(
    &mut self,
    external_port: &str,
    sender: tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  ) -> Result<(), String> {
    if !self.output_port_mapping.contains_key(external_port) {
      return Err(format!(
        "External output port '{}' is not exposed",
        external_port
      ));
    }
    self
      .connected_output_channels
      .insert(external_port.to_string(), sender);
    Ok(())
  }

  /// Returns the name of the graph.
  ///
  /// # Returns
  ///
  /// A string slice containing the graph's name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Sets the name of the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for the graph
  pub fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  /// Returns a guard over the nodes map. Use `.values()` to iterate node references.
  ///
  /// # Returns
  ///
  /// A mutex guard that derefs to the nodes `HashMap`.
  pub fn get_nodes(&self) -> std::sync::MutexGuard<'_, HashMap<String, Box<dyn Node>>> {
    self.nodes.lock().unwrap()
  }

  /// Returns a guard over the nodes map if the given node exists. Use `.get(name)` to get the node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to find
  ///
  /// # Returns
  ///
  /// `Some(guard)` if the node exists; the guard derefs to the nodes `HashMap`.
  pub fn find_node_by_name(
    &self,
    name: &str,
  ) -> Option<std::sync::MutexGuard<'_, HashMap<String, Box<dyn Node>>>> {
    let guard = self.nodes.lock().unwrap();
    if guard.contains_key(name) {
      Some(guard)
    } else {
      None
    }
  }

  /// Returns the number of nodes in the graph.
  ///
  /// # Returns
  ///
  /// The number of nodes in the graph.
  pub fn node_count(&self) -> usize {
    self.nodes.lock().unwrap().len()
  }

  /// Returns the number of edges in the graph.
  ///
  /// # Returns
  ///
  /// The number of edges in the graph.
  pub fn edge_count(&self) -> usize {
    self.edges.len()
  }

  /// Checks if a node with the given name exists in the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to check
  ///
  /// # Returns
  ///
  /// `true` if a node with the given name exists, `false` otherwise.
  pub fn has_node(&self, name: &str) -> bool {
    self.nodes.lock().unwrap().contains_key(name)
  }

  /// Checks if an edge exists between two nodes and ports.
  ///
  /// # Arguments
  ///
  /// * `source_node` - The name of the source node
  /// * `target_node` - The name of the target node
  ///
  /// # Returns
  ///
  /// `true` if an edge exists between the nodes, `false` otherwise.
  pub fn has_edge(&self, source_node: &str, target_node: &str) -> bool {
    self
      .edges
      .iter()
      .any(|e| e.source_node() == source_node && e.target_node() == target_node)
  }

  /// Adds a node to the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to the node (should match `node.name()`)
  /// * `node` - The node to add to the graph
  ///
  /// # Returns
  ///
  /// `Ok(())` if the node was added successfully, or an error if a node with
  /// the same name already exists.
  ///
  /// # Errors
  ///
  /// Returns an error string if a node with the given name already exists in the graph.
  pub fn add_node(&mut self, name: String, node: Box<dyn Node>) -> Result<(), String> {
    let mut g = self.nodes.lock().unwrap();
    if g.contains_key(&name) {
      return Err(format!("Node with name '{}' already exists", name));
    }
    g.insert(name, node);
    Ok(())
  }

  /// Removes a node from the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node to remove
  ///
  /// # Returns
  ///
  /// `Ok(())` if the node was removed successfully, or an error if the node
  /// doesn't exist or has connected edges.
  ///
  /// # Errors
  ///
  /// Returns an error string if the node doesn't exist or cannot be removed
  /// (e.g., it has connected edges).
  pub fn remove_node(&mut self, name: &str) -> Result<(), String> {
    let mut g = self.nodes.lock().unwrap();
    if !g.contains_key(name) {
      return Err(format!("Node with name '{}' does not exist", name));
    }

    let has_edges = self
      .edges
      .iter()
      .any(|e| e.source_node() == name || e.target_node() == name);

    if has_edges {
      return Err(format!(
        "Cannot remove node '{}': it has connected edges",
        name
      ));
    }

    g.remove(name);
    Ok(())
  }

  /// Returns all edges in the graph.
  ///
  /// # Returns
  ///
  /// A vector of references to all edges in the graph.
  pub fn get_edges(&self) -> Vec<&Edge> {
    self.edges.iter().collect()
  }

  /// Returns nodes that directly depend on the given node's output.
  ///
  /// Used for dependency tracking in time-range recomputation: when a node's
  /// input changes for a time range, its direct dependents may need to recompute.
  pub fn nodes_depending_on(&self, node: &str) -> Vec<String> {
    let mut deps: Vec<String> = self
      .edges
      .iter()
      .filter(|e| e.source_node() == node)
      .map(|e| e.target_node().to_string())
      .collect::<std::collections::HashSet<_>>()
      .into_iter()
      .collect();
    deps.sort();
    deps
  }

  /// Returns all nodes transitively downstream of the given node.
  ///
  /// Used for dependency tracking: when recomputing for a time range starting
  /// from a source node, this gives the set of nodes that might need to run.
  pub fn nodes_downstream_transitive(&self, node: &str) -> Vec<String> {
    let mut result = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(node.to_string());
    while let Some(n) = queue.pop_front() {
      for dep in self.nodes_depending_on(&n) {
        if result.insert(dep.clone()) {
          queue.push_back(dep);
        }
      }
    }
    result.remove(node);
    let mut out: Vec<String> = result.into_iter().collect();
    out.sort();
    out
  }

  /// Plans which nodes need recomputation for a time range.
  ///
  /// Uses [`plan_recompute`](crate::incremental::plan_recompute) with this graph's
  /// dependency structure. When `progress_handle` has per-sink frontiers (from
  /// [`execute_with_progress_per_sink`](Self::execute_with_progress_per_sink)),
  /// sinks that have already completed the time range may be excluded.
  ///
  /// # Returns
  ///
  /// A [`RecomputePlan`](crate::incremental::RecomputePlan) with node IDs to run.
  pub fn plan_recompute(
    &self,
    request: &RecomputeRequest,
    progress_handle: Option<&ProgressHandle>,
  ) -> RecomputePlan {
    let all_nodes: Vec<String> = self.nodes.lock().unwrap().keys().cloned().collect();
    let sink_keys: std::collections::HashSet<(String, String)> = self
      .output_port_mapping
      .values()
      .map(|m| (m.node.clone(), m.port.clone()))
      .collect();
    let sink_keys = if sink_keys.is_empty() {
      None
    } else {
      Some(sink_keys)
    };
    let sink_frontiers_owned = progress_handle.and_then(|h| h.sink_frontiers());
    let sink_frontiers = sink_frontiers_owned.as_ref();
    incremental_plan_recompute(
      request,
      |n| self.nodes_downstream_transitive(n),
      &all_nodes,
      sink_frontiers,
      sink_keys.as_ref(),
    )
  }

  /// Executes recomputation for the given plan.
  ///
  /// When subgraph time-scoped execution is not yet implemented, this runs the
  /// full graph via [`execute`](Self::execute). Call [`wait_for_completion`](Self::wait_for_completion)
  /// after. Use with timestamped inputs bounded to the plan's time range for
  /// correct semantics.
  ///
  /// # Note
  ///
  /// Full time-scoped execution (run only `plan.nodes` for the time range) is
  /// planned for a future release. See [incremental-recomputation.md](../docs/incremental-recomputation.md).
  pub async fn execute_recompute(
    &mut self,
    _plan: &RecomputePlan,
  ) -> Result<(), GraphExecutionError> {
    self.execute().await
  }

  /// Returns true if the graph contains at least one cycle.
  ///
  /// Uses DFS to detect back edges. Cyclic graphs require
  /// [`execute_with_rounds`](Self::execute_with_rounds) instead of [`execute`](Self::execute).
  pub fn has_cycles(&self) -> bool {
    !Self::find_feedback_edges(
      &self.nodes.lock().unwrap().keys().cloned().collect::<Vec<_>>(),
      &self.edges,
    )
    .is_empty()
  }

  /// Returns edges that are feedback edges (back edges in DFS).
  ///
  /// Removing these edges would make the graph acyclic. Used by
  /// [`execute_with_rounds`](Self::execute_with_rounds) for round-based execution.
  pub fn feedback_edges(&self) -> Vec<Edge> {
    let node_names: Vec<String> = self.nodes.lock().unwrap().keys().cloned().collect();
    Self::find_feedback_edges(&node_names, &self.edges)
  }

  /// Finds feedback edges (back edges) via DFS.
  fn find_feedback_edges(node_names: &[String], edges: &[Edge]) -> Vec<Edge> {
    let n: HashMap<&str, usize> = node_names
      .iter()
      .enumerate()
      .map(|(i, s)| (s.as_str(), i))
      .collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); node_names.len()];
    let mut edge_map: HashMap<(usize, usize), Vec<Edge>> = HashMap::new();
    for e in edges {
      if let (Some(&si), Some(&ti)) =
        (n.get(e.source_node()), n.get(e.target_node()))
      {
        if si != ti {
          adj[si].push(ti);
          edge_map.entry((si, ti)).or_default().push(e.clone());
        }
      }
    }
    let mut visited = vec![false; node_names.len()];
    let mut in_stack = vec![false; node_names.len()];
    let mut feedback = Vec::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    fn dfs(
      v: usize,
      adj: &[Vec<usize>],
      visited: &mut [bool],
      in_stack: &mut [bool],
      edge_map: &HashMap<(usize, usize), Vec<Edge>>,
      feedback: &mut Vec<Edge>,
      seen: &mut HashSet<(usize, usize)>,
    ) {
      visited[v] = true;
      in_stack[v] = true;
      for &u in &adj[v] {
        if !visited[u] {
          dfs(u, adj, visited, in_stack, edge_map, feedback, seen);
        } else if in_stack[u] && !seen.contains(&(v, u)) {
          seen.insert((v, u));
          if let Some(es) = edge_map.get(&(v, u)) {
            feedback.extend(es.iter().cloned());
          }
        }
      }
      in_stack[v] = false;
    }
    for i in 0..node_names.len() {
      if !visited[i] {
        dfs(
          i,
          &adj,
          &mut visited,
          &mut in_stack,
          &edge_map,
          &mut feedback,
          &mut seen,
        );
      }
    }
    feedback
  }

  /// Gets an edge by source and target node and port.
  ///
  /// # Arguments
  ///
  /// * `source_node` - The name of the source node
  /// * `source_port` - The name of the source output port
  /// * `target_node` - The name of the target node
  /// * `target_port` - The name of the target input port
  ///
  /// # Returns
  ///
  /// `Some(&Edge)` if an edge matching the given parameters exists, `None` otherwise.
  pub fn find_edge_by_nodes_and_ports(
    &self,
    source_node: &str,
    source_port: &str,
    target_node: &str,
    target_port: &str,
  ) -> Option<&Edge> {
    self.edges.iter().find(|e| {
      e.source_node() == source_node
        && e.source_port() == source_port
        && e.target_node() == target_node
        && e.target_port() == target_port
    })
  }

  /// Adds an edge to the graph.
  ///
  /// # Arguments
  ///
  /// * `edge` - The edge to add to the graph
  ///
  /// # Returns
  ///
  /// `Ok(())` if the edge was added successfully, or an error if the edge is invalid
  /// (e.g., nodes don't exist or ports don't exist).
  ///
  /// # Errors
  ///
  /// Returns an error string if:
  /// - The source or target node doesn't exist
  /// - The source or target port doesn't exist on the respective node
  /// - The edge would create a duplicate connection
  pub fn add_edge(&mut self, edge: Edge) -> Result<(), String> {
    let g = self.nodes.lock().unwrap();
    // Validate source node exists
    if !g.contains_key(edge.source_node()) {
      return Err(format!(
        "Source node '{}' does not exist",
        edge.source_node()
      ));
    }

    // Validate target node exists
    if !g.contains_key(edge.target_node()) {
      return Err(format!(
        "Target node '{}' does not exist",
        edge.target_node()
      ));
    }

    // Validate ports exist
    let source_node = g.get(edge.source_node()).unwrap();
    if !source_node.has_output_port(edge.source_port()) {
      return Err(format!(
        "Source node '{}' does not have output port '{}'",
        edge.source_node(),
        edge.source_port()
      ));
    }

    let target_node = g.get(edge.target_node()).unwrap();
    if !target_node.has_input_port(edge.target_port()) {
      return Err(format!(
        "Target node '{}' does not have input port '{}'",
        edge.target_node(),
        edge.target_port()
      ));
    }
    drop(g);

    // Check for duplicates
    if self
      .find_edge_by_nodes_and_ports(
        edge.source_node(),
        edge.source_port(),
        edge.target_node(),
        edge.target_port(),
      )
      .is_some()
    {
      return Err("Edge already exists".to_string());
    }

    self.edges.push(edge);
    Ok(())
  }

  /// Removes an edge from the graph.
  ///
  /// # Arguments
  ///
  /// * `source_node` - The name of the source node
  /// * `source_port` - The name of the source output port
  /// * `target_node` - The name of the target node
  /// * `target_port` - The name of the target input port
  ///
  /// # Returns
  ///
  /// `Ok(())` if the edge was removed successfully, or an error if the edge
  /// doesn't exist.
  ///
  /// # Errors
  ///
  /// Returns an error string if no edge matching the given parameters exists.
  pub fn remove_edge(
    &mut self,
    source_node: &str,
    source_port: &str,
    target_node: &str,
    target_port: &str,
  ) -> Result<(), String> {
    let index = self
      .edges
      .iter()
      .position(|e| {
        e.source_node() == source_node
          && e.source_port() == source_port
          && e.target_node() == target_node
          && e.target_port() == target_port
      })
      .ok_or_else(|| "Edge not found".to_string())?;

    self.edges.remove(index);
    Ok(())
  }

  /// Executes the graph by connecting streams between nodes.
  ///
  /// This method:
  /// 1. Routes external input streams to exposed input ports
  /// 2. Performs topological sort to determine execution order
  /// 3. For each node, collects input streams from connected upstream nodes
  /// 4. Calls `node.execute(inputs)` which returns output streams
  /// 5. Routes exposed output ports to external output senders
  /// 6. Connects output streams to downstream nodes' input streams
  /// 7. Drives all streams to completion
  ///
  /// Channels are used internally for backpressure, but are never exposed to nodes.
  /// Nodes only see and work with streams. External I/O is handled through exposed ports.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was started successfully, or an error if:
  /// - Nodes have invalid port configurations
  /// - Streams cannot be created or connected
  /// - Tasks cannot be spawned
  /// - External I/O connections are invalid
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if execution cannot be started.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::Graph;
  /// use streamweave::nodes::variable_node::VariableNode;
  /// use tokio::sync::mpsc;
  /// #[tokio::main]
  /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
  ///   let mut graph = Graph::new("g".to_string());
  ///   graph.add_node("map".to_string(), Box::new(VariableNode::new("map".to_string())))?;
  ///   graph.expose_input_port("map", "value", "configuration")?;
  ///   graph.expose_output_port("map", "out", "output")?;
  ///   let (config_tx, config_rx) = mpsc::channel(1);
  ///   let (output_tx, _output_rx) = mpsc::channel(10);
  ///   graph.connect_input_channel("configuration", config_rx)?;
  ///   graph.connect_output_channel("output", output_tx)?;
  ///   graph.execute().await.unwrap();
  ///   graph.wait_for_completion().await.unwrap();
  ///   Ok(())
  /// }
  /// ```
  pub async fn execute(&mut self) -> Result<(), GraphExecutionError> {
    // Create input streams from connected channels (merge payload-only and timestamped; timestamped stripped to payload)
    let mut external_inputs = HashMap::new();
    for (port_name, receiver_option) in &mut self.connected_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }
    for (port_name, receiver_option) in &mut self.connected_timestamped_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver)
          .map(|ts: Timestamped<Arc<dyn Any + Send + Sync>>| ts.payload);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }

    // Use connected output channels
    let external_outputs = &self.connected_output_channels;

    // Dataflow model: take nodes out so each can run in its own task; restore in wait_for_completion
    let nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };
    let (handles, nodes_restored) = self
      .run_dataflow(
        nodes,
        Some(external_inputs),
        Some(external_outputs),
        None,
        HashMap::new(),
        None,
      )
      .await?; // None = no progress, no timestamped inputs needed (already merged as payload)
    self.execution_handles.lock().await.clear();
    self.execution_handles.lock().await.extend(handles);
    self.nodes_restored_after_run = Some(nodes_restored);
    self.running.store(true, Ordering::Release);
    tracing::info!(graph_id = %self.name, "Graph execution started");
    Ok(())
  }

  /// Returns true if the graph is ready to accept work (started and running).
  ///
  /// Use for Kubernetes readiness probes: return 200 when ready, 503 otherwise.
  /// The graph is ready after [`execute`](Self::execute) has started and until
  /// [`stop`](Self::stop) or [`wait_for_completion`](Self::wait_for_completion) finishes.
  pub fn is_ready(&self) -> bool {
    self.running.load(Ordering::Acquire)
  }

  /// Returns true if the graph is alive (always true for an existing graph).
  ///
  /// Use for Kubernetes liveness probes. Process-level liveness is typically
  /// handled by the HTTP server; this indicates the graph instance exists.
  pub fn is_live(&self) -> bool {
    true
  }

  /// Starts graph execution in deterministic mode.
  ///
  /// Same as [`execute()`](Self::execute), but uses [`ExecutionMode::Deterministic`]:
  /// nodes are started in topological order for reproducible runs. Use in tests
  /// and when reproducibility matters.
  pub async fn execute_deterministic(&mut self) -> Result<(), GraphExecutionError> {
    self.set_execution_mode(ExecutionMode::Deterministic);
    self.execute().await
  }

  /// Executes a cyclic graph with Timely-style round-based semantics.
  ///
  /// For graphs with cycles (feedback edges), runs up to `max_rounds` rounds. Round 0
  /// uses external inputs; rounds 1..N inject the previous round's feedback output.
  /// Feedback edges are identified via DFS and "cut" at round boundaries.
  ///
  /// For acyclic graphs, delegates to [`execute()`](Self::execute). Call
  /// [`connect_input_channel`](Self::connect_input_channel) and
  /// [`connect_output_channel`](Self::connect_output_channel) before calling.
  ///
  /// See [cyclic-iterative-dataflows.md](../docs/cyclic-iterative-dataflows.md) Option B.
  pub async fn execute_with_rounds(&mut self, max_rounds: usize) -> Result<(), GraphExecutionError> {
    if !self.has_cycles() {
      return self.execute().await;
    }
    if max_rounds == 0 {
      return Ok(());
    }
    let feedback_edges = self.feedback_edges();
    if feedback_edges.is_empty() {
      return self.execute().await;
    }

    let mut external_inputs = HashMap::new();
    for (port_name, receiver_option) in &mut self.connected_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }
    for (port_name, receiver_option) in &mut self.connected_timestamped_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver)
          .map(|ts: Timestamped<Arc<dyn Any + Send + Sync>>| ts.payload);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }
    let external_outputs = self.connected_output_channels.clone();

    let mut nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };
    let mut feedback_buffers: Vec<Vec<Arc<dyn Any + Send + Sync>>> =
      vec![Vec::new(); feedback_edges.len()];

    for round in 0..max_rounds {
      let mut inject_and_capture = Vec::new();
      let mut capture_rxs: Vec<tokio::sync::mpsc::Receiver<EdgeMessage>> = Vec::new();
      for (i, edge) in feedback_edges.iter().enumerate() {
        let (inject_tx, inject_rx) =
          tokio::sync::mpsc::channel::<EdgeMessage>(DATAFLOW_CHANNEL_CAPACITY);
        let (capture_tx, capture_rx) =
          tokio::sync::mpsc::channel::<EdgeMessage>(DATAFLOW_CHANNEL_CAPACITY);
        inject_and_capture.push((edge.clone(), inject_rx, capture_tx));
        capture_rxs.push(capture_rx);

        if round == 0 {
          drop(inject_tx);
        } else {
          let buf = std::mem::take(&mut feedback_buffers[i]);
          tokio::spawn(async move {
            for item in buf {
              let _ = inject_tx.send(EdgeMessage::PayloadOnly(item)).await;
            }
            drop(inject_tx);
          });
        }
      }
      let feedback_config = RoundFeedbackConfig { inject_and_capture };

      let ext_inputs = if round == 0 {
        Some(std::mem::take(&mut external_inputs))
      } else {
        let mut empty = HashMap::new();
        for port in self.input_port_mapping.keys() {
          let stream = Box::pin(tokio_stream::iter(std::iter::empty::<Arc<dyn Any + Send + Sync>>()))
            as crate::node::InputStream;
          empty.insert(port.clone(), stream);
        }
        Some(empty)
      };
      let ext_ts: HashMap<String, tokio::sync::mpsc::Receiver<Timestamped<Arc<dyn Any + Send + Sync>>>> = HashMap::new();

      let (handles, nodes_restored) = self
        .run_dataflow(
          std::mem::take(&mut nodes),
          ext_inputs,
          Some(&external_outputs),
          None,
          ext_ts,
          Some(feedback_config),
        )
        .await?;

      for (_, h) in handles {
        h.await
          .map_err(|e| -> GraphExecutionError { format!("Round {} task failed: {}", round, e).into() })??;
      }
      {
        let mut guard = nodes_restored.lock().await;
        nodes = std::mem::take(&mut *guard);
      }

      for (i, mut capture_rx) in capture_rxs.into_iter().enumerate() {
        let mut buf = Vec::new();
        while let Some(msg) = capture_rx.recv().await {
          if let EdgeMessage::PayloadOnly(p) = msg {
            buf.push(p);
          }
        }
        feedback_buffers[i] = buf;
      }
    }

    let mut g = self.nodes.lock().unwrap();
    *g = nodes;
    self.running.store(false, Ordering::Release);
    tracing::info!(graph_id = %self.name, rounds = max_rounds, "Cyclic graph execution completed");
    Ok(())
  }

  /// Starts graph execution with progress tracking.
  ///
  /// Same as [`execute()`](Self::execute), but the completed frontier is advanced
  /// when items reach exposed output ports (sinks). Returns a [`ProgressHandle`]
  /// so callers can observe progress (e.g. `less_than(t)`, `less_equal(t)`).
  ///
  /// Uses a single shared frontier; for graphs with multiple sinks, consider
  /// [`execute_with_progress_per_sink`](Self::execute_with_progress_per_sink) so that
  /// graph-level progress is the minimum over per-sink frontiers.
  ///
  /// # Returns
  ///
  /// `Ok(ProgressHandle)` so the caller can poll progress; the graph runs as with `execute()`.
  pub async fn execute_with_progress(
    &mut self,
  ) -> Result<ProgressHandle, GraphExecutionError> {
    let mut external_inputs = HashMap::new();
    let mut external_timestamped_inputs = HashMap::new();
    for (port_name, receiver_option) in &mut self.connected_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }
    for (port_name, receiver_option) in &mut self.connected_timestamped_input_channels {
      if let Some(receiver) = receiver_option.take() {
        external_timestamped_inputs.insert(port_name.clone(), receiver);
      }
    }
    let external_outputs = &self.connected_output_channels;
    let nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };
    let frontier = Arc::new(CompletedFrontier::new());
    let progress_handle = ProgressHandle::new(Arc::clone(&frontier));
    let progress_config = Some(ProgressFrontierConfig::Single(frontier));
    let (handles, nodes_restored) = self
      .run_dataflow(
        nodes,
        Some(external_inputs),
        Some(external_outputs),
        progress_config,
        external_timestamped_inputs,
        None,
      )
      .await?;
    self.execution_handles.lock().await.clear();
    self.execution_handles.lock().await.extend(handles);
    self.nodes_restored_after_run = Some(nodes_restored);
    self.running.store(true, Ordering::Release);
    Ok(progress_handle)
  }

  /// Starts graph execution with progress tracking and per-sink frontiers.
  ///
  /// Same as [`execute_with_progress`](Self::execute_with_progress), but maintains
  /// one frontier per exposed output port. Graph-level progress is the **minimum**
  /// over all sink frontiers: "all sinks have completed up to T." Use this when
  /// different sinks consume at different rates and you need accurate progress.
  ///
  /// # Returns
  ///
  /// `Ok(ProgressHandle)` with `frontier()` returning min over per-sink frontiers.
  pub async fn execute_with_progress_per_sink(
    &mut self,
  ) -> Result<ProgressHandle, GraphExecutionError> {
    let mut external_inputs = HashMap::new();
    let mut external_timestamped_inputs = HashMap::new();
    for (port_name, receiver_option) in &mut self.connected_input_channels {
      if let Some(receiver) = receiver_option.take() {
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver);
        let pinned_stream = Box::pin(stream) as crate::node::InputStream;
        external_inputs.insert(port_name.clone(), pinned_stream);
      }
    }
    for (port_name, receiver_option) in &mut self.connected_timestamped_input_channels {
      if let Some(receiver) = receiver_option.take() {
        external_timestamped_inputs.insert(port_name.clone(), receiver);
      }
    }
    let external_outputs = &self.connected_output_channels;
    let nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };
    let mut per_sink = HashMap::new();
    let mut frontiers = Vec::new();
    let mut sink_keys = Vec::new();
    for mapping in self.output_port_mapping.values() {
      let key = (mapping.node.clone(), mapping.port.clone());
      if !per_sink.contains_key(&key) {
        let f = Arc::new(CompletedFrontier::new());
        per_sink.insert(key.clone(), Arc::clone(&f));
        frontiers.push(f);
        sink_keys.push(key);
      }
    }
    let progress_config = if per_sink.is_empty() {
      None
    } else {
      Some(ProgressFrontierConfig::PerSink(per_sink))
    };
    let progress_handle = if frontiers.is_empty() {
      ProgressHandle::new(Arc::new(CompletedFrontier::new()))
    } else {
      ProgressHandle::from_sink_frontiers(frontiers, sink_keys)
    };
    let (handles, nodes_restored) = self
      .run_dataflow(
        nodes,
        Some(external_inputs),
        Some(external_outputs),
        progress_config,
        external_timestamped_inputs,
        None,
      )
      .await?;
    self.execution_handles.lock().await.clear();
    self.execution_handles.lock().await.extend(handles);
    self.nodes_restored_after_run = Some(nodes_restored);
    self.running.store(true, Ordering::Release);
    Ok(progress_handle)
  }

  /// Returns node names in topological order (sources first).
  /// Nodes with no incoming edges come first; ties broken by name.
  fn topological_order(
    nodes: &HashMap<String, Box<dyn Node>>,
    edges: &[&Edge],
  ) -> Vec<String> {
    let mut in_degree: HashMap<String, usize> = nodes.keys().map(|n| (n.clone(), 0)).collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for e in edges {
      if e.source_node() != e.target_node() {
        in_degree
          .entry(e.target_node().to_string())
          .and_modify(|d| *d += 1)
          .or_insert(1);
        outgoing
          .entry(e.source_node().to_string())
          .or_default()
          .push(e.target_node().to_string());
      }
    }
    let mut queue: VecDeque<String> = in_degree
      .iter()
      .filter(|kv| *kv.1 == 0)
      .map(|(n, _)| n.clone())
      .collect();
    queue.make_contiguous().sort();
    let mut result = Vec::new();
    while let Some(n) = queue.pop_front() {
      result.push(n.clone());
      for m in outgoing.get(&n).into_iter().flatten().cloned().collect::<Vec<_>>() {
        if let Some(d) = in_degree.get_mut(&m) {
          *d = d.saturating_sub(1);
          if *d == 0 {
            queue.push_back(m);
          }
        }
      }
      queue.make_contiguous().sort();
    }
    // Add any remaining nodes (e.g. in cycles) in sorted order
    for n in in_degree.keys().cloned().collect::<Vec<_>>() {
      if !result.contains(&n) {
        result.push(n);
      }
    }
    result.sort();
    result
  }

  /// Dataflow execution: one channel per edge, one task per node. Supports cycles.
  /// When `progress_config` is `Some`, items on edges carry logical times and the
  /// frontier(s) are advanced when items reach exposed outputs (sinks).
  /// When `external_timestamped_inputs` has a port, user-provided time is used instead of counter.
  /// When `feedback` is `Some`, feedback edges use inject/capture instead of normal channels (round-based execution).
  /// Returns (handles, nodes_restored) so callers can await and restore nodes.
  async fn run_dataflow(
    &self,
    mut nodes: HashMap<String, Box<dyn Node>>,
    external_inputs: Option<HashMap<String, crate::node::InputStream>>,
    external_outputs: Option<
      &HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
    >,
    progress_config: Option<ProgressFrontierConfig>,
    mut external_timestamped_inputs: HashMap<
      String,
      tokio::sync::mpsc::Receiver<Timestamped<Arc<dyn Any + Send + Sync>>>,
    >,
    feedback: Option<RoundFeedbackConfig>,
  ) -> Result<(Vec<TaskHandle>, NodesRestoredMap), GraphExecutionError> {
    if let Some(config) = self.shard_config {
      crate::metrics::record_shard_assignment(config.shard_id, config.total_shards);
    }
    let edges = self.get_edges();
    let feedback_edges_set: HashSet<(String, String, String, String)> = feedback
      .as_ref()
      .map(|f| {
        f.inject_and_capture
          .iter()
          .map(|(e, _, _)| {
            (
              e.source_node().to_string(),
              e.source_port().to_string(),
              e.target_node().to_string(),
              e.target_port().to_string(),
            )
          })
          .collect()
      })
      .unwrap_or_default();

    let initial_time = self
      .restored_position
      .map(|t| t.as_u64() + 1)
      .unwrap_or(0);
    let time_counter = progress_config
      .as_ref()
      .map(|_| Arc::new(AtomicU64::new(initial_time)));

    // One channel per edge: (target_node, target_port) -> receiver
    let mut input_rx: HashMap<(String, String), tokio::sync::mpsc::Receiver<EdgeMessage>> =
      HashMap::new();
    // (source_node, source_port) -> list of senders (one per edge)
    let mut output_txs: HashMap<(String, String), Vec<tokio::sync::mpsc::Sender<EdgeMessage>>> =
      HashMap::new();

    for edge in &edges {
      let key = (
        edge.source_node().to_string(),
        edge.source_port().to_string(),
        edge.target_node().to_string(),
        edge.target_port().to_string(),
      );
      if feedback_edges_set.contains(&key) {
        continue;
      }
      let (tx, rx) = tokio::sync::mpsc::channel::<EdgeMessage>(DATAFLOW_CHANNEL_CAPACITY);
      input_rx.insert(
        (
          edge.target_node().to_string(),
          edge.target_port().to_string(),
        ),
        rx,
      );
      output_txs
        .entry((
          edge.source_node().to_string(),
          edge.source_port().to_string(),
        ))
        .or_default()
        .push(tx);
    }

    if let Some(mut f) = feedback {
      for (e, inject_rx, capture_tx) in f.inject_and_capture.drain(..) {
        input_rx.insert(
          (e.target_node().to_string(), e.target_port().to_string()),
          inject_rx,
        );
        output_txs
          .entry((e.source_node().to_string(), e.source_port().to_string()))
          .or_default()
          .push(capture_tx);
      }
    }

    let nodes_restored = Arc::new(Mutex::new(HashMap::new()));
    let mut all_handles: Vec<TaskHandle> = Vec::new();
    let deterministic = self.execution_mode == ExecutionMode::Deterministic;

    // Route external input streams into channels. Prefer timestamped (user-provided time) over payload-only (counter).
    // In deterministic mode, process ports in sorted order.
    let mut ts_inputs: Vec<_> = external_timestamped_inputs.drain().collect();
    if deterministic {
      ts_inputs.sort_by(|a, b| a.0.cmp(&b.0));
    }
    for (external_port, mut receiver) in ts_inputs {
      if let Some(mapping) = self.input_port_mapping.get(&external_port) {
        let (tx, rx) = tokio::sync::mpsc::channel::<EdgeMessage>(DATAFLOW_CHANNEL_CAPACITY);
        input_rx.insert((mapping.node.clone(), mapping.port.clone()), rx);
        let stop_signal = Arc::clone(&self.stop_signal);
        let counter = time_counter.clone();
        let handle = tokio::spawn(async move {
          loop {
            tokio::select! {
              _ = stop_signal.notified() => break,
              item = receiver.recv() => {
                match item {
                  Some(ts) => {
                    let msg = match &counter {
                      Some(_) => EdgeMessage::WithTime(ts.payload, ts.time),
                      None => EdgeMessage::PayloadOnly(ts.payload),
                    };
                    if tx.send(msg).await.is_err() {
                      break;
                    }
                  }
                  None => break,
                }
              }
            }
          }
          drop(tx);
          Ok(()) as Result<(), GraphExecutionError>
        });
        all_handles.push((None, handle));
      }
    }
    // Route payload-only external inputs (use counter when progress enabled)
    // In deterministic mode, process ports in sorted order.
    if let Some(mut external_inputs) = external_inputs {
      let mut inputs: Vec<_> = external_inputs.drain().collect();
      if deterministic {
        inputs.sort_by(|a, b| a.0.cmp(&b.0));
      }
      for (external_port, stream) in inputs {
        if let Some(mapping) = self.input_port_mapping.get(&external_port) {
          let (tx, rx) = tokio::sync::mpsc::channel::<EdgeMessage>(DATAFLOW_CHANNEL_CAPACITY);
          input_rx.insert((mapping.node.clone(), mapping.port.clone()), rx);
          let stop_signal = Arc::clone(&self.stop_signal);
          let counter = time_counter.clone();
          let handle = tokio::spawn(async move {
            let mut stream = stream;
            loop {
              tokio::select! {
                _ = stop_signal.notified() => break,
                item = stream.next() => {
                  match item {
                    Some(item) => {
                      let msg = match &counter {
                        Some(c) => EdgeMessage::WithTime(
                          item,
                          LogicalTime::new(c.fetch_add(1, Ordering::SeqCst)),
                        ),
                        None => EdgeMessage::PayloadOnly(item),
                      };
                      if tx.send(msg).await.is_err() {
                        break;
                      }
                    }
                    None => break,
                  }
                }
              }
            }
            drop(tx);
            Ok(()) as Result<(), GraphExecutionError>
          });
          all_handles.push((None, handle));
        }
      }
    }

    let stop_signal = Arc::clone(&self.stop_signal);
    let output_port_mapping = self.output_port_mapping.clone();

    // In deterministic mode, process nodes in topological order.
    let node_order: Vec<String> = if deterministic {
      Self::topological_order(&nodes, &edges)
    } else {
      nodes.keys().cloned().collect()
    };

    for node_name in node_order {
      let node = match nodes.remove(&node_name) {
        Some(n) => n,
        None => continue,
      };
      let node_input_keys: Vec<(String, String)> = input_rx
        .keys()
        .filter(|(n, _)| n == &node_name)
        .cloned()
        .collect();
      let mut input_streams: InputStreams = HashMap::new();
      for (_, port) in node_input_keys {
        if let Some(rx) = input_rx.remove(&(node_name.clone(), port.clone())) {
          let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|m| match m {
            EdgeMessage::PayloadOnly(p) => p,
            EdgeMessage::WithTime(p, _) => p,
          });
          input_streams.insert(port, Box::pin(stream) as crate::node::InputStream);
        }
      }
      let output_txs_for_node: HashMap<String, Vec<tokio::sync::mpsc::Sender<EdgeMessage>>> =
        output_txs
          .iter()
          .filter(|((n, _), _)| n == &node_name)
          .map(|(k, v)| (k.1.clone(), v.clone()))
          .collect();
      let nodes_restored = Arc::clone(&nodes_restored);
      let stop_signal = Arc::clone(&stop_signal);
      let output_port_mapping = output_port_mapping.clone();
      let external_outputs = external_outputs.cloned();
      let progress_config = progress_config.clone();
      let counter = time_counter.clone();
      let node_name_for_handle = node_name.clone();

      let handle = tokio::spawn(async move {
        let node_outputs = match node.execute(input_streams).await {
          Ok(o) => o,
          Err(e) => {
            let _ = nodes_restored.lock().await.insert(node_name.clone(), node);
            return Err(format!("Node '{}' execution error: {}", node_name, e).into());
          }
        };

        nodes_restored.lock().await.insert(node_name.clone(), node);

        let mut forwarder_handles = Vec::new();
        for (port_name, stream) in node_outputs {
          let internal_senders = output_txs_for_node
            .get(&port_name)
            .cloned()
            .unwrap_or_default();
          let (external_tx, frontier_for_port) = {
            let is_exposed = output_port_mapping
              .values()
              .any(|mapping| mapping.node == node_name && mapping.port == port_name);
            if is_exposed
              && let Some(ref outputs) = external_outputs
              && let Some((external_port, _)) = output_port_mapping
                .iter()
                .find(|(_, m)| m.node == node_name && m.port == port_name)
              && let Some(tx) = outputs.get(external_port)
            {
              let frontier_opt = match &progress_config {
                Some(ProgressFrontierConfig::Single(f)) => Some(Arc::clone(f)),
                Some(ProgressFrontierConfig::PerSink(m)) => {
                  m.get(&(node_name.clone(), port_name.clone())).cloned()
                }
                None => None,
              };
              (Some(tx.clone()), frontier_opt)
            } else {
              (None, None)
            }
          };

          let stop_signal = Arc::clone(&stop_signal);
          let counter = counter.clone();
          let handle = tokio::spawn(async move {
            let mut stream = stream;
            loop {
              tokio::select! {
                _ = stop_signal.notified() => break,
                item = stream.next() => {
                  match item {
                    Some(item) => {
                      let (msg, time_opt) = match &counter {
                        Some(c) => {
                          let t = LogicalTime::new(c.fetch_add(1, Ordering::SeqCst));
                          (EdgeMessage::WithTime(item.clone(), t), Some(t))
                        }
                        None => (EdgeMessage::PayloadOnly(item.clone()), None),
                      };
                      for tx in &internal_senders {
                        if tx.send(msg.clone()).await.is_err() {
                          return Ok(());
                        }
                      }
                      if let Some(ext_tx) = &external_tx {
                        if ext_tx.send(item).await.is_err() {
                          return Ok(());
                        }
                        if let (Some(f), Some(t)) = (frontier_for_port.as_ref(), time_opt) {
                          f.advance_to(t);
                        }
                      }
                    }
                    None => break,
                  }
                }
              }
            }
            Ok(()) as Result<(), GraphExecutionError>
          });
          forwarder_handles.push(handle);
        }

        for h in forwarder_handles {
          let _ = h.await;
        }
        Ok(())
      });
      all_handles.push((Some(node_name_for_handle), handle));
    }

    Ok((all_handles, nodes_restored))
  }

  /// Starts graph execution.
  ///
  /// Sets the execution state to running. The graph will begin processing when
  /// data arrives on input ports (pull-based model).
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was started successfully.
  pub fn start(&self) {
    self.execution_state.store(1, Ordering::Release); // 1 = running
    self.pause_signal.notify_waiters(); // Resume if paused
  }

  /// Pauses graph execution.
  ///
  /// The graph will stop processing new data but maintains its current state.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was paused successfully.
  pub fn pause(&self) {
    self.execution_state.store(2, Ordering::Release); // 2 = paused
  }

  /// Resumes graph execution.
  ///
  /// Resumes processing after a pause.
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was resumed successfully.
  pub fn resume(&self) {
    self.execution_state.store(1, Ordering::Release); // 1 = running
    self.pause_signal.notify_waiters();
  }

  /// Stops graph execution and clears all state.
  ///
  /// This method:
  /// 1. Signals all nodes to stop processing
  /// 2. Clears all execution handles
  /// 3. Resets execution state to None
  /// 4. All data flowing through the graph is discarded
  ///
  /// # Returns
  ///
  /// `Ok(())` if execution was stopped successfully, or an error if stopping failed.
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if execution cannot be stopped gracefully.
  pub async fn stop(&self) -> Result<(), GraphExecutionError> {
    tracing::info!(graph_id = %self.name, "Graph stop requested");
    // Notify all tasks to stop
    self.stop_signal.notify_waiters();

    // Wait for all tasks to complete
    let handles = {
      let mut handles_guard = self.execution_handles.lock().await;
      std::mem::take(&mut *handles_guard)
    };

    for (_node_id_opt, handle) in handles {
      let _ = handle.await;
    }

    // Reset execution state
    self.execution_state.store(0, Ordering::Release); // 0 = stopped
    self.running.store(false, Ordering::Release);

    // Clear execution handles
    self.execution_handles.lock().await.clear();

    tracing::info!(graph_id = %self.name, "Graph stopped");
    Ok(())
  }

  /// Internal execution method that routes external streams to internal nodes.
  ///
  /// This is called when Graph is used as a Node in another graph.
  /// Uses the same dataflow execution as execute() so nested graphs support cycles.
  async fn execute_internal(
    &self,
    external_inputs: Option<InputStreams>,
  ) -> Result<OutputStreams, GraphExecutionError> {
    type Payload = Arc<dyn Any + Send + Sync>;

    // Take nodes out of the graph so we can run dataflow
    let nodes = {
      let mut g = self.nodes.lock().unwrap();
      std::mem::take(&mut *g)
    };

    // Create a channel for each exposed output port; run_dataflow will send to these
    let mut external_output_txs: HashMap<String, tokio::sync::mpsc::Sender<Payload>> =
      HashMap::new();
    let mut output_rxs: HashMap<String, tokio::sync::mpsc::Receiver<Payload>> = HashMap::new();
    for external_port in self.output_port_mapping.keys() {
      let (tx, rx) = tokio::sync::mpsc::channel(DATAFLOW_CHANNEL_CAPACITY);
      external_output_txs.insert(external_port.clone(), tx);
      output_rxs.insert(external_port.clone(), rx);
    }

    let (handles, nodes_restored) = self
      .run_dataflow(nodes, external_inputs, Some(&external_output_txs), None, HashMap::new(), None)
      .await?;

    for (_node_id_opt, handle) in handles {
      handle.await??;
    }

    // Restore nodes into the graph
    {
      let mut restored = nodes_restored.lock().await;
      *self.nodes.lock().unwrap() = std::mem::take(&mut *restored);
    }

    // Build OutputStreams from the receivers we created for exposed outputs
    let mut external_outputs: OutputStreams = HashMap::new();
    for (external_port, rx) in output_rxs {
      let stream =
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)) as crate::node::OutputStream;
      external_outputs.insert(external_port, stream);
    }

    Ok(external_outputs)
  }

  /// Waits for all nodes in the graph to complete execution.
  ///
  /// This method blocks until all node tasks have finished. Use this after calling
  /// `execute()` to wait for the graph to finish processing. When using the dataflow
  /// execution model, nodes are restored into the graph after all tasks complete.
  ///
  /// # Returns
  ///
  /// `Ok(())` if all nodes completed successfully, or an error if any node failed.
  ///
  /// # Errors
  ///
  /// Returns `GraphExecutionError` if any node execution failed or if waiting timed out.
  pub async fn wait_for_completion(&mut self) -> Result<(), GraphExecutionError> {
    let handles = {
      let mut handles_guard = self.execution_handles.lock().await;
      std::mem::take(&mut *handles_guard)
    };

    let mut first_error: Option<GraphExecutionError> = None;

    // Wait for all tasks to complete (await all so nodes are restored for retry)
    for (node_id_opt, handle) in handles {
      match handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
          if first_error.is_none() {
            let node_id = node_id_opt.as_deref().unwrap_or("?");
            crate::metrics::record_node_error(&self.name, node_id);
            tracing::error!(
              graph_id = %self.name,
              node_id = %node_id,
              error = %e,
              "Node execution failed"
            );
            if let (Some(tx), Some(node_id)) = (&self.failure_tx, &node_id_opt) {
              let _ = tx.try_send(FailureReport {
                node_id: node_id.clone(),
                error: e.to_string(),
              });
            }
            first_error = Some(e.into());
          }
        }
        Err(join_err) => {
          if first_error.is_none() {
            let node_id = node_id_opt.as_deref().unwrap_or("?");
            crate::metrics::record_node_error(&self.name, node_id);
            tracing::error!(
              graph_id = %self.name,
              node_id = %node_id,
              error = %join_err,
              "Node task panicked"
            );
            if let (Some(tx), Some(node_id)) = (&self.failure_tx, &node_id_opt) {
              let _ = tx.try_send(FailureReport {
                node_id: node_id.clone(),
                error: format!("task panicked: {}", join_err),
              });
            }
            first_error = Some(join_err.to_string().into());
          }
        }
      }
    }

    // Restore nodes after dataflow execution so the graph can be reused (including for retry)
    if let Some(restore) = self.nodes_restored_after_run.take() {
      let mut map = restore.lock().await;
      *self.nodes.lock().unwrap() = std::mem::take(&mut *map);
    }

    self.running.store(false, Ordering::Release);

    if let Some(e) = first_error {
      Err(e)
    } else {
      Ok(())
    }
  }

  /// Executes the graph with supervision: on node failure, applies the configured
  /// policy (restart graph up to N times, or stop).
  ///
  /// Uses **graph-level restart**: when a node fails and the policy says Restart,
  /// the whole graph is restarted. Per-node restart requires reconnecting inputs;
  /// use the `reconnect` closure to provide fresh channels for each attempt.
  ///
  /// # Arguments
  ///
  /// * `reconnect` - Called before each execution attempt (including the first).
  ///   Must set up input/output channels via `connect_input_channel`, etc.
  ///
  /// # Returns
  ///
  /// `Ok(())` if the graph completed successfully, or an error if:
  /// - Policy says Stop/Escalate
  /// - Max restarts exceeded
  /// - Reconnect or execution fails
  pub async fn execute_with_supervision<F>(
    &mut self,
    mut reconnect: F,
  ) -> Result<(), GraphExecutionError>
  where
    F: FnMut(&mut Graph) -> Result<(), GraphExecutionError>,
  {
    let mut failure_rx = self.enable_failure_reporting();
    let mut restart_count: HashMap<String, u32> = HashMap::new();

    loop {
      reconnect(self)?;

      if let Err(e) = self.execute().await {
        return Err(e);
      }

      match self.wait_for_completion().await {
        Ok(()) => return Ok(()),
        Err(exec_err) => {
          // Receive the failure report (sent before wait_for_completion returned)
          let report = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            failure_rx.recv(),
          )
          .await
          .ok()
          .flatten();

          let node_id = report
            .as_ref()
            .map(|r| r.node_id.as_str())
            .unwrap_or("?");

          let policy = self
            .node_supervision_policies
            .get(node_id)
            .cloned()
            .unwrap_or_else(|| self.default_supervision_policy.clone());

          let restart_key = match policy.on_failure {
            FailureAction::Restart => node_id.to_string(),
            FailureAction::RestartGroup => self
              .node_supervision_groups
              .get(node_id)
              .cloned()
              .map(|g| format!("group:{}", g))
              .unwrap_or_else(|| node_id.to_string()),
            FailureAction::Stop | FailureAction::Escalate => node_id.to_string(),
          };
          let count = restart_count.entry(restart_key.clone()).or_insert(0);
          *count += 1;

          match policy.on_failure {
            FailureAction::Stop => {
              tracing::error!(
                node_id = %node_id,
                "Node failed (Stop policy), stopping graph"
              );
              return Err(exec_err);
            }
            FailureAction::Escalate => {
              tracing::error!(
                node_id = %node_id,
                "Node failed (Escalate policy), escalating to graph level"
              );
              return Err(exec_err);
            }
            FailureAction::Restart | FailureAction::RestartGroup => {
              if let Some(max) = policy.max_restarts {
                if *count > max {
                  tracing::error!(
                    node_id = %node_id,
                    restart_count = *count,
                    max_restarts = max,
                    "Max restarts exceeded, stopping"
                  );
                  return Err(exec_err);
                }
              }
              let scope_msg = match policy.on_failure {
                FailureAction::RestartGroup => "restarting group",
                _ => "restarting graph",
              };
              tracing::warn!(
                node_id = %node_id,
                error = %report.as_ref().map(|r| r.error.as_str()).unwrap_or(""),
                restart_count = *count,
                %scope_msg,
                "Node failed"
              );
              tokio::time::sleep(policy.restart_backoff).await;
            }
          }
        }
      }
    }
  }
}

/// Helper function to perform topological sort of nodes in a graph.
///
/// Returns nodes in execution order (sources first, sinks last).
/// This ensures that when we execute nodes, all upstream nodes have already
/// produced their output streams.
pub fn topological_sort(
  nodes: &[&dyn Node],
  edges: &[&Edge],
) -> Result<Vec<String>, GraphExecutionError> {
  let mut in_degree: HashMap<String, usize> = HashMap::new();
  let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

  // Initialize in-degree for all nodes
  for node in nodes {
    in_degree.insert(node.name().to_string(), 0);
    adjacency.insert(node.name().to_string(), Vec::new());
  }

  // Build adjacency list and calculate in-degrees
  for edge in edges {
    let source = edge.source_node().to_string();
    let target = edge.target_node().to_string();

    adjacency.get_mut(&source).unwrap().push(target.clone());
    *in_degree.get_mut(&target).unwrap() += 1;
  }

  // Kahn's algorithm for topological sort
  let mut queue: VecDeque<String> = VecDeque::new();
  for (node_name, &degree) in &in_degree {
    if degree == 0 {
      queue.push_back(node_name.clone());
    }
  }

  let mut result = Vec::new();
  while let Some(node_name) = queue.pop_front() {
    result.push(node_name.clone());

    if let Some(neighbors) = adjacency.get(&node_name) {
      for neighbor in neighbors {
        let degree = in_degree.get_mut(neighbor).unwrap();
        *degree -= 1;
        if *degree == 0 {
          queue.push_back(neighbor.clone());
        }
      }
    }
  }

  // Check for cycles
  if result.len() != nodes.len() {
    return Err("Graph contains cycles".into());
  }

  Ok(result)
}

// ============================================================================
// Node Implementation for Graph
// ============================================================================

#[async_trait]
impl Node for Graph {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &self.input_port_names
  }

  fn output_port_names(&self) -> &[String] {
    &self.output_port_names
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.input_port_names.contains(&name.to_string())
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.output_port_names.contains(&name.to_string())
  }

  fn execute(
    &self,
    inputs: InputStreams,
  ) -> Pin<Box<dyn Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
    Box::pin(async move {
      // Execute the graph as a node, routing external streams to internal nodes
      self
        .execute_internal(Some(inputs))
        .await
        .map_err(|e| format!("Graph execution error: {}", e).into())
    })
  }
}
