//! Recovery strategies for handling worker failures.

use crate::coordinator::{Coordinator, CoordinatorError, WorkerInfo};
use crate::fault_tolerance::checkpoint::{Checkpoint, CheckpointError, CheckpointStore};
use crate::worker::WorkerId;
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fault_tolerance::checkpoint::InMemoryCheckpointStore;

  // Mock coordinator for testing
  struct MockCoordinator {
    workers: std::collections::HashMap<WorkerId, WorkerInfo>,
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

    async fn get_workers(
      &self,
    ) -> Result<std::collections::HashMap<WorkerId, WorkerInfo>, CoordinatorError> {
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

  use proptest::prelude::*;

  fn recovery_strategy_strategy() -> impl Strategy<Value = RecoveryStrategy> {
    prop::sample::select(&[
      RecoveryStrategy::FromCheckpoint,
      RecoveryStrategy::FromBeginning,
      RecoveryStrategy::Skip,
      RecoveryStrategy::Manual,
    ])
  }

  async fn test_recovery_manager_creation_async(strategy: RecoveryStrategy) {
    let coordinator = MockCoordinator {
      workers: std::collections::HashMap::new(),
    };
    let store = InMemoryCheckpointStore::new();
    let manager = RecoveryManager::new(coordinator, store, strategy);
    assert_eq!(manager.strategy, strategy);
  }

  proptest! {
    #[test]
    fn test_recovery_manager_creation(strategy in recovery_strategy_strategy()) {
      let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
      rt.block_on(test_recovery_manager_creation_async(strategy));
    }
  }
}
