//! Tests for distributed failure_detector module

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use streamweave::distributed::coordinator::{Coordinator, CoordinatorError, WorkerInfo};
use streamweave::distributed::failure_detector::{
  FailureDetector, FailureDetectorConfig, FailureEvent,
};
use streamweave::distributed::worker::{WorkerId, WorkerState};

// Mock coordinator for testing
struct MockCoordinator {
  workers: HashMap<WorkerId, WorkerInfo>,
}

impl MockCoordinator {
  fn new() -> Self {
    Self {
      workers: HashMap::new(),
    }
  }

  fn add_worker(&mut self, worker_id: WorkerId, healthy: bool) {
    let last_heartbeat = if healthy {
      Some(chrono::Utc::now())
    } else {
      Some(chrono::Utc::now() - chrono::Duration::seconds(100))
    };

    self.workers.insert(
      worker_id.clone(),
      WorkerInfo {
        worker_id: worker_id.clone(),
        state: WorkerState::Ready,
        address: "127.0.0.1:8080".parse().unwrap(),
        last_heartbeat,
        assigned_partitions: vec![],
        capacity: 10,
        current_load: 0,
      },
    );
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

#[tokio::test]
async fn test_failure_detector_config_default() {
  let config = FailureDetectorConfig::default();
  assert_eq!(config.heartbeat_timeout, Duration::from_secs(30));
  assert_eq!(config.check_interval, Duration::from_secs(10));
  assert_eq!(config.failure_threshold, 3);
}

#[tokio::test]
async fn test_failure_detector_new() {
  let coordinator = MockCoordinator::new();
  let config = FailureDetectorConfig::default();
  let detector = FailureDetector::new(coordinator, config);

  // Should create successfully
  assert!(true);
}

#[tokio::test]
async fn test_failure_detector_detect_failures_healthy() {
  let mut coordinator = MockCoordinator::new();
  let worker_id = WorkerId::new("worker-1".to_string());
  coordinator.add_worker(worker_id.clone(), true);

  let config = FailureDetectorConfig {
    heartbeat_timeout: Duration::from_secs(30),
    check_interval: Duration::from_secs(10),
    failure_threshold: 3,
  };
  let mut detector = FailureDetector::new(coordinator, config);

  let events = detector.detect_failures().await.unwrap();
  // Should have no failure events for healthy worker
  assert!(events.is_empty());
}

#[tokio::test]
async fn test_failure_detector_failed_workers() {
  let coordinator = MockCoordinator::new();
  let config = FailureDetectorConfig::default();
  let detector = FailureDetector::new(coordinator, config);

  // Initially no failed workers
  let failed = detector.failed_workers();
  assert_eq!(failed.len(), 0);
}

#[tokio::test]
async fn test_failure_detector_is_failed() {
  let coordinator = MockCoordinator::new();
  let config = FailureDetectorConfig::default();
  let detector = FailureDetector::new(coordinator, config);
  let worker_id = WorkerId::new("worker-1".to_string());

  // Worker not marked as failed initially
  assert!(!detector.is_failed(&worker_id));
}

#[test]
fn test_failure_event_variants() {
  let worker_id = WorkerId::new("worker-1".to_string());

  assert!(matches!(
    FailureEvent::WorkerFailed(worker_id.clone()),
    FailureEvent::WorkerFailed(_)
  ));

  assert!(matches!(
    FailureEvent::WorkerRecovered(worker_id.clone()),
    FailureEvent::WorkerRecovered(_)
  ));

  assert!(matches!(
    FailureEvent::MultipleFailures(vec![worker_id.clone()]),
    FailureEvent::MultipleFailures(_)
  ));
}
