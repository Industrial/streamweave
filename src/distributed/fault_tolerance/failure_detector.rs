//! Failure detection for workers and coordinators.

use crate::distributed::coordinator::{Coordinator, CoordinatorError};
use crate::distributed::worker::WorkerId;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for failure detection.
#[derive(Debug, Clone)]
pub struct FailureDetectorConfig {
  /// Heartbeat timeout (worker considered dead if no heartbeat within this time).
  pub heartbeat_timeout: Duration,
  /// Interval for checking worker health.
  pub check_interval: Duration,
  /// Number of consecutive failures before declaring worker dead.
  pub failure_threshold: usize,
}

impl Default for FailureDetectorConfig {
  fn default() -> Self {
    Self {
      heartbeat_timeout: Duration::from_secs(30),
      check_interval: Duration::from_secs(10),
      failure_threshold: 3,
    }
  }
}

/// Failure event types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureEvent {
  /// Worker detected as failed.
  WorkerFailed(WorkerId),
  /// Worker recovered after previous failure.
  WorkerRecovered(WorkerId),
  /// Multiple workers failed (potential coordinator issue).
  MultipleFailures(Vec<WorkerId>),
}

/// Failure detector for monitoring worker health.
pub struct FailureDetector<C>
where
  C: Coordinator,
{
  /// Coordinator for accessing worker information.
  coordinator: C,
  /// Failure detection configuration.
  config: FailureDetectorConfig,
  /// Track consecutive failures per worker.
  failure_counts: HashMap<WorkerId, usize>,
  /// Previously detected failed workers.
  failed_workers: HashMap<WorkerId, chrono::DateTime<Utc>>,
}

impl<C> FailureDetector<C>
where
  C: Coordinator,
{
  /// Creates a new failure detector.
  #[must_use]
  pub fn new(coordinator: C, config: FailureDetectorConfig) -> Self {
    Self {
      coordinator,
      config,
      failure_counts: HashMap::new(),
      failed_workers: HashMap::new(),
    }
  }

  /// Checks for worker failures and returns failure events.
  pub async fn detect_failures(&mut self) -> Result<Vec<FailureEvent>, CoordinatorError> {
    let mut events = Vec::new();
    let workers = self.coordinator.get_workers().await?;

    for (worker_id, worker_info) in &workers {
      // Check if worker is healthy
      let is_healthy = worker_info.is_healthy(self.config.heartbeat_timeout);

      if is_healthy {
        // Worker is healthy - check if it was previously failed
        if self.failed_workers.remove(worker_id).is_some() {
          events.push(FailureEvent::WorkerRecovered(worker_id.clone()));
        }
        // Reset failure count
        self.failure_counts.remove(worker_id);
      } else {
        // Worker appears unhealthy
        let failure_count = self.failure_counts.entry(worker_id.clone()).or_insert(0);
        *failure_count += 1;

        // Check if threshold reached
        if *failure_count >= self.config.failure_threshold {
          // Mark as failed if not already
          if !self.failed_workers.contains_key(worker_id) {
            self.failed_workers.insert(worker_id.clone(), Utc::now());
            events.push(FailureEvent::WorkerFailed(worker_id.clone()));
          }
        }
      }
    }

    // Check for multiple failures
    if events
      .iter()
      .filter(|e| matches!(e, FailureEvent::WorkerFailed(_)))
      .count()
      > 1
    {
      let failed: Vec<WorkerId> = events
        .iter()
        .filter_map(|e| {
          if let FailureEvent::WorkerFailed(id) = e {
            Some(id.clone())
          } else {
            None
          }
        })
        .collect();
      events.push(FailureEvent::MultipleFailures(failed));
    }

    Ok(events)
  }

  /// Gets list of currently failed workers.
  #[must_use]
  pub fn failed_workers(&self) -> Vec<WorkerId> {
    self.failed_workers.keys().cloned().collect()
  }

  /// Checks if a specific worker is marked as failed.
  #[must_use]
  pub fn is_failed(&self, worker_id: &WorkerId) -> bool {
    self.failed_workers.contains_key(worker_id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::distributed::coordinator::WorkerInfo;
  use std::collections::HashMap;

  // Mock coordinator for testing
  struct MockCoordinator {
    workers: HashMap<WorkerId, WorkerInfo>,
  }

  #[async_trait::async_trait]
  impl Coordinator for MockCoordinator {
    async fn register_worker(&mut self, _worker_info: WorkerInfo) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn unregister_worker(&mut self, _worker_id: &WorkerId) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn update_heartbeat(&mut self, _worker_id: &WorkerId) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn assign_partitions(
      &mut self,
      _worker_id: &WorkerId,
      _partitions: Vec<usize>,
    ) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn get_workers(&self) -> Result<HashMap<WorkerId, WorkerInfo>, CoordinatorError> {
      Ok(self.workers.clone())
    }

    async fn get_worker(&self, worker_id: &WorkerId) -> Result<WorkerInfo, CoordinatorError> {
      self
        .workers
        .get(worker_id)
        .cloned()
        .ok_or(CoordinatorError::WorkerNotFound(worker_id.clone()))
    }

    async fn rebalance(&mut self) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn start(&mut self) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn stop(&mut self) -> Result<(), CoordinatorError> {
      Ok(())
    }

    async fn health_check(&mut self) -> Result<Vec<WorkerId>, CoordinatorError> {
      Ok(vec![])
    }
  }

  #[tokio::test]
  async fn test_failure_detector_creation() {
    let coordinator = MockCoordinator {
      workers: HashMap::new(),
    };
    let detector = FailureDetector::new(coordinator, FailureDetectorConfig::default());
    assert_eq!(detector.config.failure_threshold, 3);
  }
}
