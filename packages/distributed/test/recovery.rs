use proptest::prelude::*;
use streamweave_distributed::InMemoryCheckpointStore;
use streamweave_distributed::WorkerId;
use streamweave_distributed::{Coordinator, CoordinatorError, WorkerInfo};
use streamweave_distributed::{RecoveryManager, RecoveryStrategy};

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

#[tokio::test]
async fn test_recovery_manager_recover_failed_worker() {
  use std::collections::HashMap;
  use std::net::SocketAddr;
  use std::time::Duration;
  use streamweave_distributed::WorkerInfo;
  use streamweave_distributed::WorkerState;

  let mut workers = HashMap::new();
  let failed_worker_id = WorkerId::new("failed-worker".to_string());
  let healthy_worker_id = WorkerId::new("healthy-worker".to_string());

  let failed_worker = WorkerInfo {
    worker_id: failed_worker_id.clone(),
    state: WorkerState::Error,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now() - chrono::Duration::seconds(100)),
    assigned_partitions: vec![0, 1, 2],
    capacity: 10,
    current_load: 0,
  };

  let healthy_worker = WorkerInfo {
    worker_id: healthy_worker_id.clone(),
    state: WorkerState::Ready,
    address: "127.0.0.1:8081".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now()),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  workers.insert(failed_worker_id.clone(), failed_worker);
  workers.insert(healthy_worker_id.clone(), healthy_worker);

  let coordinator = MockCoordinator { workers };
  let store = InMemoryCheckpointStore::new();
  let mut manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::FromCheckpoint);

  let redistributed = manager
    .recover_failed_worker(&failed_worker_id)
    .await
    .unwrap();
  assert_eq!(redistributed.len(), 3);
  assert!(redistributed.contains(&0));
  assert!(redistributed.contains(&1));
  assert!(redistributed.contains(&2));
}

#[tokio::test]
async fn test_recovery_manager_recover_state_from_checkpoint() {
  use std::collections::HashMap;
  use streamweave_distributed::Checkpoint;

  let coordinator = MockCoordinator {
    workers: std::collections::HashMap::new(),
  };
  let store = InMemoryCheckpointStore::new();

  // Save a checkpoint
  let checkpoint = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![1, 2, 3],
    metadata: HashMap::new(),
  };
  store.save(&checkpoint).await.unwrap();

  let manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::FromCheckpoint);
  let recovered = manager.recover_state("worker-1", 0).await.unwrap();
  assert!(recovered.is_some());
  assert_eq!(recovered.unwrap().offset, 100);
}

#[tokio::test]
async fn test_recovery_manager_recover_state_from_beginning() {
  let coordinator = MockCoordinator {
    workers: std::collections::HashMap::new(),
  };
  let store = InMemoryCheckpointStore::new();
  let manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::FromBeginning);

  let recovered = manager.recover_state("worker-1", 0).await.unwrap();
  assert!(recovered.is_none());
}

#[tokio::test]
async fn test_recovery_manager_recover_state_skip() {
  let coordinator = MockCoordinator {
    workers: std::collections::HashMap::new(),
  };
  let store = InMemoryCheckpointStore::new();
  let manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::Skip);

  let recovered = manager.recover_state("worker-1", 0).await.unwrap();
  assert!(recovered.is_none());
}

#[tokio::test]
async fn test_recovery_manager_recover_state_manual() {
  let coordinator = MockCoordinator {
    workers: std::collections::HashMap::new(),
  };
  let store = InMemoryCheckpointStore::new();
  let manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::Manual);

  let recovered = manager.recover_state("worker-1", 0).await.unwrap();
  assert!(recovered.is_none());
}

#[tokio::test]
async fn test_recovery_manager_get_recovery_offset_with_checkpoint() {
  use std::collections::HashMap;
  use streamweave_distributed::Checkpoint;

  let coordinator = MockCoordinator {
    workers: std::collections::HashMap::new(),
  };
  let store = InMemoryCheckpointStore::new();

  let checkpoint = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 200,
    state_data: vec![],
    metadata: HashMap::new(),
  };
  store.save(&checkpoint).await.unwrap();

  let manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::FromCheckpoint);
  let offset = manager.get_recovery_offset("worker-1", 0).await.unwrap();
  assert_eq!(offset, 200);
}

#[tokio::test]
async fn test_recovery_manager_get_recovery_offset_no_checkpoint() {
  let coordinator = MockCoordinator {
    workers: std::collections::HashMap::new(),
  };
  let store = InMemoryCheckpointStore::new();
  let manager = RecoveryManager::new(coordinator, store, RecoveryStrategy::FromCheckpoint);

  let offset = manager.get_recovery_offset("worker-1", 0).await.unwrap();
  assert_eq!(offset, 0); // Should default to 0
}

#[test]
fn test_recovery_strategy_variants() {
  let strategies = vec![
    RecoveryStrategy::FromCheckpoint,
    RecoveryStrategy::FromBeginning,
    RecoveryStrategy::Skip,
    RecoveryStrategy::Manual,
  ];

  for strategy in strategies {
    let debug_str = format!("{:?}", strategy);
    assert!(!debug_str.is_empty());
  }
}
