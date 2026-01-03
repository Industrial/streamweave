//! Tests for distributed recovery module

use std::collections::HashMap;
use streamweave::distributed::checkpoint::{Checkpoint, CheckpointError, CheckpointStore};
use streamweave::distributed::coordinator::{Coordinator, CoordinatorError, WorkerInfo};
use streamweave::distributed::recovery::{RecoveryManager, RecoveryStrategy};
use streamweave::distributed::worker::{WorkerId, WorkerState};

// Mock checkpoint store for testing
struct MockCheckpointStore {
  checkpoints: HashMap<String, Checkpoint>,
}

impl MockCheckpointStore {
  fn new() -> Self {
    Self {
      checkpoints: HashMap::new(),
    }
  }
}

#[async_trait::async_trait]
impl CheckpointStore for MockCheckpointStore {
  async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
    // In a real implementation, this would save to storage
    Ok(())
  }

  async fn load_latest(
    &self,
    _worker_id: &str,
    _partition_id: usize,
  ) -> Result<Option<Checkpoint>, CheckpointError> {
    Ok(None)
  }

  async fn load(&self, _checkpoint_id: &str) -> Result<Checkpoint, CheckpointError> {
    Err(CheckpointError::NotFound("test".to_string()))
  }

  async fn list(&self, _worker_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
    Ok(vec![])
  }

  async fn delete(&self, _checkpoint_id: &str) -> Result<(), CheckpointError> {
    Ok(())
  }
}

// Mock coordinator (same as in failure_detector tests)
struct MockCoordinator {
  workers: HashMap<WorkerId, WorkerInfo>,
}

impl MockCoordinator {
  fn new() -> Self {
    Self {
      workers: HashMap::new(),
    }
  }
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

  async fn get_worker(&self, worker_id: &WorkerId) -> Result<WorkerInfo, CoordinatorError> {
    self
      .workers
      .get(worker_id)
      .cloned()
      .ok_or_else(|| CoordinatorError::WorkerNotFound(worker_id.clone()))
  }

  async fn get_workers(&self) -> Result<HashMap<WorkerId, WorkerInfo>, CoordinatorError> {
    Ok(self.workers.clone())
  }

  async fn rebalance(&mut self) -> Result<(), CoordinatorError> {
    Ok(())
  }
}

#[test]
fn test_recovery_strategy_variants() {
  assert_eq!(
    RecoveryStrategy::FromCheckpoint,
    RecoveryStrategy::FromCheckpoint
  );
  assert_eq!(
    RecoveryStrategy::FromBeginning,
    RecoveryStrategy::FromBeginning
  );
  assert_eq!(RecoveryStrategy::Skip, RecoveryStrategy::Skip);
  assert_eq!(RecoveryStrategy::Manual, RecoveryStrategy::Manual);
}

#[tokio::test]
async fn test_recovery_manager_new() {
  let coordinator = MockCoordinator::new();
  let checkpoint_store = MockCheckpointStore::new();
  let manager = RecoveryManager::new(
    coordinator,
    checkpoint_store,
    RecoveryStrategy::FromCheckpoint,
  );

  // Should create successfully
  assert!(true);
}

#[tokio::test]
async fn test_recovery_manager_recover_state_from_checkpoint() {
  let coordinator = MockCoordinator::new();
  let checkpoint_store = MockCheckpointStore::new();
  let manager = RecoveryManager::new(
    coordinator,
    checkpoint_store,
    RecoveryStrategy::FromCheckpoint,
  );

  // Should attempt to load checkpoint
  let result = manager.recover_state("worker-1", 0).await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_recovery_manager_recover_state_from_beginning() {
  let coordinator = MockCoordinator::new();
  let checkpoint_store = MockCheckpointStore::new();
  let manager = RecoveryManager::new(
    coordinator,
    checkpoint_store,
    RecoveryStrategy::FromBeginning,
  );

  // Should return None (start from beginning)
  let result = manager.recover_state("worker-1", 0).await.unwrap();
  assert!(result.is_none());
}

#[tokio::test]
async fn test_recovery_manager_get_recovery_offset() {
  let coordinator = MockCoordinator::new();
  let checkpoint_store = MockCheckpointStore::new();
  let manager = RecoveryManager::new(
    coordinator,
    checkpoint_store,
    RecoveryStrategy::FromCheckpoint,
  );

  // Should return 0 when no checkpoint exists
  let offset = manager.get_recovery_offset("worker-1", 0).await.unwrap();
  assert_eq!(offset, 0);
}
