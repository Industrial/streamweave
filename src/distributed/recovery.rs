//! Recovery strategies for handling worker failures.

use super::checkpoint::{Checkpoint, CheckpointError, CheckpointStore};
use super::coordinator::{Coordinator, CoordinatorError, WorkerInfo};
use super::worker::WorkerId;
use std::collections::HashMap;

/// Recovery strategy for handling failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
  /// Restart processing from the latest checkpoint.
  FromCheckpoint,
  /// Restart processing from the beginning.
  FromBeginning,
  /// Skip the failed partition (data loss).
  Skip,
  /// Manual intervention required.
  Manual,
}

/// Recovery manager for handling worker failures and state recovery.
pub struct RecoveryManager<C, S>
where
  C: Coordinator,
  S: CheckpointStore,
{
  /// Coordinator for managing workers.
  coordinator: C,
  /// Checkpoint store for state recovery.
  checkpoint_store: S,
  /// Recovery strategy.
  strategy: RecoveryStrategy,
  /// Track recovered partitions.
  recovered_partitions: HashMap<usize, WorkerId>,
}

impl<C, S> RecoveryManager<C, S>
where
  C: Coordinator,
  S: CheckpointStore,
{
  /// Creates a new recovery manager.
  #[must_use]
  pub fn new(coordinator: C, checkpoint_store: S, strategy: RecoveryStrategy) -> Self {
    Self {
      coordinator,
      checkpoint_store,
      strategy,
      recovered_partitions: HashMap::new(),
    }
  }

  /// Recovers partitions from a failed worker by redistributing to healthy workers.
  pub async fn recover_failed_worker(
    &mut self,
    failed_worker_id: &WorkerId,
  ) -> Result<Vec<usize>, CoordinatorError> {
    // Get information about the failed worker
    let failed_worker = self.coordinator.get_worker(failed_worker_id).await?;
    let partitions = failed_worker.assigned_partitions.clone();

    // Find healthy workers with capacity
    let workers = self.coordinator.get_workers().await?;
    let healthy_workers: Vec<&WorkerInfo> = workers
      .values()
      .filter(|w| {
        w.worker_id != *failed_worker_id
          && w.is_healthy(std::time::Duration::from_secs(30))
          && w.has_capacity()
      })
      .collect();

    if healthy_workers.is_empty() {
      return Err(CoordinatorError::StateError(
        "No healthy workers available for recovery".to_string(),
      ));
    }

    // Redistribute partitions to healthy workers
    let mut redistributed = Vec::new();
    for (i, partition) in partitions.iter().enumerate() {
      let target_worker = &healthy_workers[i % healthy_workers.len()];

      // Assign partition to new worker
      self
        .coordinator
        .assign_partitions(&target_worker.worker_id, vec![*partition])
        .await?;

      redistributed.push(*partition);
      self
        .recovered_partitions
        .insert(*partition, target_worker.worker_id.clone());
    }

    // Rebalance if configured
    let _ = self.coordinator.rebalance().await;

    Ok(redistributed)
  }

  /// Recovers state from checkpoint for a partition.
  pub async fn recover_state(
    &self,
    worker_id: &str,
    partition_id: usize,
  ) -> Result<Option<Checkpoint>, CheckpointError> {
    match self.strategy {
      RecoveryStrategy::FromCheckpoint => {
        self
          .checkpoint_store
          .load_latest(worker_id, partition_id)
          .await
      }
      RecoveryStrategy::FromBeginning | RecoveryStrategy::Skip | RecoveryStrategy::Manual => {
        Ok(None)
      }
    }
  }

  /// Gets the recovery offset for a partition (for resuming processing).
  pub async fn get_recovery_offset(
    &self,
    worker_id: &str,
    partition_id: usize,
  ) -> Result<u64, CheckpointError> {
    match self.recover_state(worker_id, partition_id).await? {
      Some(checkpoint) => Ok(checkpoint.offset),
      None => Ok(0), // Start from beginning
    }
  }
}
