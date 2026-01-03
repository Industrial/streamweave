//! Worker node definitions for distributed processing.
//!
//! Defines the worker role, state, and configuration for processing
//! stream data in a distributed environment.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

/// Unique identifier for a worker node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerId(String);

impl WorkerId {
  /// Creates a new worker ID from a string.
  #[must_use]
  pub fn new(id: String) -> Self {
    Self(id)
  }

  /// Returns the ID as a string slice.
  #[must_use]
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl From<String> for WorkerId {
  fn from(s: String) -> Self {
    WorkerId::new(s)
  }
}

impl fmt::Display for WorkerId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Worker state in the distributed system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WorkerState {
  /// Worker is initializing.
  #[default]
  Initializing,
  /// Worker is registered and ready for tasks.
  Ready,
  /// Worker is processing a task.
  Processing,
  /// Worker is in an error state.
  Error,
  /// Worker is shutting down.
  ShuttingDown,
  /// Worker has been terminated.
  Terminated,
}

/// Configuration for a worker node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
  /// Worker identifier.
  pub worker_id: WorkerId,
  /// Address where worker listens for coordinator connections.
  pub bind_address: SocketAddr,
  /// Coordinator address for registration.
  pub coordinator_address: SocketAddr,
  /// Heartbeat interval.
  pub heartbeat_interval: Duration,
  /// Timeout for coordinator communication.
  pub communication_timeout: Duration,
  /// Maximum number of concurrent tasks.
  pub max_concurrent_tasks: usize,
  /// Partition assignments for this worker.
  pub assigned_partitions: Vec<usize>,
}

impl Default for WorkerConfig {
  fn default() -> Self {
    Self {
      worker_id: WorkerId::new(format!("worker-{}", chrono::Utc::now().timestamp_millis())),
      bind_address: "0.0.0.0:0".parse().unwrap(),
      coordinator_address: "127.0.0.1:8080".parse().unwrap(),
      heartbeat_interval: Duration::from_secs(5),
      communication_timeout: Duration::from_secs(30),
      max_concurrent_tasks: 10,
      assigned_partitions: Vec::new(),
    }
  }
}

/// Worker-related errors.
#[derive(Debug)]
pub enum WorkerError {
  /// Worker registration failed.
  RegistrationFailed(String),
  /// Communication error with coordinator.
  Communication(String),
  /// Invalid task assignment.
  InvalidTask(String),
  /// Pipeline execution error.
  PipelineError(String),
  /// Worker state error.
  StateError(String),
  /// Other worker error.
  Other(String),
}

impl fmt::Display for WorkerError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      WorkerError::RegistrationFailed(msg) => write!(f, "Registration failed: {}", msg),
      WorkerError::Communication(msg) => write!(f, "Communication error: {}", msg),
      WorkerError::InvalidTask(msg) => write!(f, "Invalid task: {}", msg),
      WorkerError::PipelineError(msg) => write!(f, "Pipeline error: {}", msg),
      WorkerError::StateError(msg) => write!(f, "State error: {}", msg),
      WorkerError::Other(msg) => write!(f, "Worker error: {}", msg),
    }
  }
}

impl std::error::Error for WorkerError {}

/// Trait for worker nodes in distributed processing.
///
/// Workers are responsible for:
/// - Registering with the coordinator
/// - Receiving and processing task assignments
/// - Executing pipelines on assigned partitions
/// - Sending heartbeats and status updates
/// - Handling failures and recovery
#[async_trait::async_trait]
pub trait Worker: Send + Sync {
  /// Returns the worker ID.
  fn worker_id(&self) -> &WorkerId;

  /// Returns the current worker state.
  fn state(&self) -> WorkerState;

  /// Registers this worker with the coordinator.
  async fn register(&mut self) -> Result<(), WorkerError>;

  /// Starts the worker event loop.
  async fn start(&mut self) -> Result<(), WorkerError>;

  /// Stops the worker gracefully.
  async fn stop(&mut self) -> Result<(), WorkerError>;

  /// Processes an assigned task/partition.
  async fn process_task(&mut self, task_id: String, partition: usize) -> Result<(), WorkerError>;

  /// Sends a heartbeat to the coordinator.
  async fn send_heartbeat(&mut self) -> Result<(), WorkerError>;
}
