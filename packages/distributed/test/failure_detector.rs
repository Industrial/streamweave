use proptest::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use streamweave_distributed::WorkerId;
use streamweave_distributed::{Coordinator, CoordinatorError, WorkerInfo};
use streamweave_distributed::{FailureDetector, FailureDetectorConfig, FailureEvent};

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

async fn test_failure_detector_creation_async(
  heartbeat_timeout_secs: u64,
  check_interval_secs: u64,
  failure_threshold: usize,
) {
  let coordinator = MockCoordinator {
    workers: HashMap::new(),
  };
  let config = FailureDetectorConfig {
    heartbeat_timeout: Duration::from_secs(heartbeat_timeout_secs),
    check_interval: Duration::from_secs(check_interval_secs),
    failure_threshold,
  };
  let detector = FailureDetector::new(coordinator, config.clone());
  assert_eq!(
    detector.config.heartbeat_timeout,
    Duration::from_secs(heartbeat_timeout_secs)
  );
  assert_eq!(
    detector.config.check_interval,
    Duration::from_secs(check_interval_secs)
  );
  assert_eq!(detector.config.failure_threshold, failure_threshold);
}

proptest! {
  #[test]
  fn test_failure_detector_creation(
    heartbeat_timeout_secs in 1u64..3600,
    check_interval_secs in 1u64..3600,
    failure_threshold in 1usize..100,
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_failure_detector_creation_async(
      heartbeat_timeout_secs,
      check_interval_secs,
      failure_threshold,
    ));
  }

  #[test]
  fn test_failure_detector_config_default(_ in any::<u8>()) {
    let config = FailureDetectorConfig::default();
    prop_assert_eq!(config.heartbeat_timeout, Duration::from_secs(30));
    prop_assert_eq!(config.check_interval, Duration::from_secs(10));
    prop_assert_eq!(config.failure_threshold, 3);
  }
}

#[tokio::test]
async fn test_failure_detector_detect_failures() {
  let mut workers = HashMap::new();
  let worker_id1 = WorkerId::new("worker-1".to_string());
  let worker_id2 = WorkerId::new("worker-2".to_string());

  // Healthy worker
  let healthy_worker = WorkerInfo {
    worker_id: worker_id1.clone(),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now()),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  // Unhealthy worker (old heartbeat)
  let unhealthy_worker = WorkerInfo {
    worker_id: worker_id2.clone(),
    state: WorkerState::Ready,
    address: "127.0.0.1:8081".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now() - chrono::Duration::seconds(100)),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  workers.insert(worker_id1.clone(), healthy_worker);
  workers.insert(worker_id2.clone(), unhealthy_worker);

  let coordinator = MockCoordinator { workers };
  let config = FailureDetectorConfig {
    heartbeat_timeout: Duration::from_secs(30),
    check_interval: Duration::from_secs(10),
    failure_threshold: 1, // Low threshold for testing
  };

  let mut detector = FailureDetector::new(coordinator, config);
  let events = detector.detect_failures().await.unwrap();

  // Should detect worker-2 as failed
  assert!(!events.is_empty());
  assert!(
    events
      .iter()
      .any(|e| matches!(e, FailureEvent::WorkerFailed(id) if id == &worker_id2))
  );
}

#[tokio::test]
async fn test_failure_detector_worker_recovered() {
  let mut workers = HashMap::new();
  let worker_id = WorkerId::new("worker-1".to_string());

  // Worker that was previously failed but is now healthy
  let recovered_worker = WorkerInfo {
    worker_id: worker_id.clone(),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now()),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  workers.insert(worker_id.clone(), recovered_worker);

  let coordinator = MockCoordinator { workers };
  let config = FailureDetectorConfig::default();
  let mut detector = FailureDetector::new(coordinator, config);

  // Manually mark as failed first
  detector
    .failed_workers
    .insert(worker_id.clone(), chrono::Utc::now());

  // Now detect - should see recovery event
  let events = detector.detect_failures().await.unwrap();
  assert!(
    events
      .iter()
      .any(|e| matches!(e, FailureEvent::WorkerRecovered(id) if id == &worker_id))
  );
}

#[tokio::test]
async fn test_failure_detector_multiple_failures() {
  let mut workers = HashMap::new();
  let worker_id1 = WorkerId::new("worker-1".to_string());
  let worker_id2 = WorkerId::new("worker-2".to_string());

  // Two unhealthy workers
  let unhealthy1 = WorkerInfo {
    worker_id: worker_id1.clone(),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now() - chrono::Duration::seconds(100)),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  let unhealthy2 = WorkerInfo {
    worker_id: worker_id2.clone(),
    state: WorkerState::Ready,
    address: "127.0.0.1:8081".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now() - chrono::Duration::seconds(100)),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  workers.insert(worker_id1.clone(), unhealthy1);
  workers.insert(worker_id2.clone(), unhealthy2);

  let coordinator = MockCoordinator { workers };
  let config = FailureDetectorConfig {
    heartbeat_timeout: Duration::from_secs(30),
    check_interval: Duration::from_secs(10),
    failure_threshold: 1,
  };

  let mut detector = FailureDetector::new(coordinator, config);
  let events = detector.detect_failures().await.unwrap();

  // Should have MultipleFailures event
  assert!(
    events
      .iter()
      .any(|e| matches!(e, FailureEvent::MultipleFailures(ids) if ids.len() >= 2))
  );
}

#[test]
fn test_failure_detector_failed_workers() {
  let coordinator = MockCoordinator {
    workers: HashMap::new(),
  };
  let config = FailureDetectorConfig::default();
  let mut detector = FailureDetector::new(coordinator, config);

  let worker_id = WorkerId::new("worker-1".to_string());
  detector
    .failed_workers
    .insert(worker_id.clone(), chrono::Utc::now());

  let failed = detector.failed_workers();
  assert_eq!(failed.len(), 1);
  assert_eq!(failed[0], worker_id);
}

#[test]
fn test_failure_detector_is_failed() {
  let coordinator = MockCoordinator {
    workers: HashMap::new(),
  };
  let config = FailureDetectorConfig::default();
  let mut detector = FailureDetector::new(coordinator, config);

  let worker_id = WorkerId::new("worker-1".to_string());
  assert!(!detector.is_failed(&worker_id));

  detector
    .failed_workers
    .insert(worker_id.clone(), chrono::Utc::now());
  assert!(detector.is_failed(&worker_id));
}

#[test]
fn test_failure_event_variants() {
  let worker_id = WorkerId::new("worker-1".to_string());

  let events = vec![
    FailureEvent::WorkerFailed(worker_id.clone()),
    FailureEvent::WorkerRecovered(worker_id.clone()),
    FailureEvent::MultipleFailures(vec![worker_id.clone()]),
  ];

  for event in events {
    let debug_str = format!("{:?}", event);
    assert!(!debug_str.is_empty());
  }
}

#[test]
fn test_failure_event_equality() {
  let worker_id = WorkerId::new("worker-1".to_string());
  let event1 = FailureEvent::WorkerFailed(worker_id.clone());
  let event2 = FailureEvent::WorkerFailed(worker_id.clone());
  assert_eq!(event1, event2);
}
