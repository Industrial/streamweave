//! Coordinator node definitions for distributed processing.
//!
//! Defines the coordinator role, responsibilities, and configuration for
//! managing worker nodes and distributing tasks in a distributed stream
//! processing system.

use crate::distributed::worker::{WorkerId, WorkerState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

/// Configuration for a coordinator node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
  /// Address where coordinator listens for worker connections.
  pub bind_address: SocketAddr,
  /// Heartbeat timeout (how long to wait before considering worker dead).
  pub heartbeat_timeout: Duration,
  /// Interval for checking worker health.
  pub health_check_interval: Duration,
  /// Maximum number of workers to manage.
  pub max_workers: usize,
  /// Rebalance strategy when workers join/leave.
  pub rebalance_strategy: RebalanceStrategy,
  /// Enable coordinator failover (requires external consensus).
  pub enable_failover: bool,
}

/// Strategy for rebalancing partitions when workers change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RebalanceStrategy {
  /// Immediate rebalance (may cause temporary disruption).
  Immediate,
  /// Gradual rebalance (migrate partitions one at a time).
  #[default]
  Gradual,
  /// No automatic rebalance (manual intervention required).
  Manual,
}

impl Default for CoordinatorConfig {
  fn default() -> Self {
    Self {
      bind_address: "0.0.0.0:8080".parse().unwrap(),
      heartbeat_timeout: Duration::from_secs(30),
      health_check_interval: Duration::from_secs(10),
      max_workers: 100,
      rebalance_strategy: RebalanceStrategy::default(),
      enable_failover: false,
    }
  }
}

/// Information about a registered worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
  /// Worker identifier.
  pub worker_id: WorkerId,
  /// Current worker state.
  pub state: WorkerState,
  /// Worker network address.
  pub address: SocketAddr,
  /// Last heartbeat timestamp.
  pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
  /// Assigned partition indices.
  pub assigned_partitions: Vec<usize>,
  /// Worker capacity (max concurrent tasks).
  pub capacity: usize,
  /// Current load (active tasks).
  pub current_load: usize,
}

impl WorkerInfo {
  /// Checks if worker is considered healthy (recent heartbeat).
  #[must_use]
  pub fn is_healthy(&self, timeout: Duration) -> bool {
    if let Some(last) = self.last_heartbeat {
      let now = chrono::Utc::now();
      let elapsed = now.signed_duration_since(last);
      elapsed.to_std().is_ok_and(|d| d < timeout)
    } else {
      false
    }
  }

  /// Checks if worker has capacity for new tasks.
  #[must_use]
  pub fn has_capacity(&self) -> bool {
    self.current_load < self.capacity
  }
}

/// Coordinator-related errors.
#[derive(Debug)]
pub enum CoordinatorError {
  /// Worker registration failed.
  RegistrationFailed(String),
  /// Worker not found.
  WorkerNotFound(WorkerId),
  /// Communication error with worker.
  Communication(String),
  /// Invalid partition assignment.
  InvalidPartition(String),
  /// Rebalancing failed.
  RebalanceFailed(String),
  /// Coordinator state error.
  StateError(String),
  /// Other coordinator error.
  Other(String),
}

impl fmt::Display for CoordinatorError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CoordinatorError::RegistrationFailed(msg) => write!(f, "Registration failed: {}", msg),
      CoordinatorError::WorkerNotFound(id) => write!(f, "Worker not found: {}", id),
      CoordinatorError::Communication(msg) => write!(f, "Communication error: {}", msg),
      CoordinatorError::InvalidPartition(msg) => write!(f, "Invalid partition: {}", msg),
      CoordinatorError::RebalanceFailed(msg) => write!(f, "Rebalance failed: {}", msg),
      CoordinatorError::StateError(msg) => write!(f, "State error: {}", msg),
      CoordinatorError::Other(msg) => write!(f, "Coordinator error: {}", msg),
    }
  }
}

impl std::error::Error for CoordinatorError {}

/// Trait for coordinator nodes in distributed processing.
///
/// Coordinators are responsible for:
/// - Managing worker registration and lifecycle
/// - Assigning partitions to workers
/// - Monitoring worker health via heartbeats
/// - Rebalancing partitions when workers join/leave
/// - Coordinating checkpointing and recovery
/// - Handling worker failures and task redistribution
#[async_trait::async_trait]
pub trait Coordinator: Send + Sync {
  /// Registers a new worker.
  async fn register_worker(&mut self, worker_info: WorkerInfo) -> Result<(), CoordinatorError>;

  /// Unregisters a worker.
  async fn unregister_worker(&mut self, worker_id: &WorkerId) -> Result<(), CoordinatorError>;

  /// Updates worker heartbeat.
  async fn update_heartbeat(&mut self, worker_id: &WorkerId) -> Result<(), CoordinatorError>;

  /// Assigns partitions to a worker.
  async fn assign_partitions(
    &mut self,
    worker_id: &WorkerId,
    partitions: Vec<usize>,
  ) -> Result<(), CoordinatorError>;

  /// Gets all registered workers.
  async fn get_workers(&self) -> Result<HashMap<WorkerId, WorkerInfo>, CoordinatorError>;

  /// Gets information about a specific worker.
  async fn get_worker(&self, worker_id: &WorkerId) -> Result<WorkerInfo, CoordinatorError>;

  /// Triggers rebalancing of partitions across workers.
  async fn rebalance(&mut self) -> Result<(), CoordinatorError>;

  /// Starts the coordinator service.
  async fn start(&mut self) -> Result<(), CoordinatorError>;

  /// Stops the coordinator service.
  async fn stop(&mut self) -> Result<(), CoordinatorError>;

  /// Checks health of all workers and removes unhealthy ones.
  async fn health_check(&mut self) -> Result<Vec<WorkerId>, CoordinatorError>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_coordinator_config_default() {
    let config = CoordinatorConfig::default();
    assert_eq!(config.heartbeat_timeout, Duration::from_secs(30));
    assert_eq!(config.max_workers, 100);
    assert_eq!(config.rebalance_strategy, RebalanceStrategy::Gradual);
  }

  #[test]
  fn test_worker_info_health() {
    let mut info = WorkerInfo {
      worker_id: WorkerId::new("worker-1".to_string()),
      state: WorkerState::Ready,
      address: "127.0.0.1:8080".parse().unwrap(),
      last_heartbeat: Some(chrono::Utc::now()),
      assigned_partitions: vec![0, 1],
      capacity: 10,
      current_load: 5,
    };

    // Should be healthy with recent heartbeat
    assert!(info.is_healthy(Duration::from_secs(60)));
    assert!(info.has_capacity());

    // Update load to capacity
    info.current_load = 10;
    assert!(!info.has_capacity());
  }
}
