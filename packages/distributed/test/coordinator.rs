use proptest::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;
use streamweave_distributed::{CoordinatorConfig, CoordinatorError, RebalanceStrategy, WorkerInfo};
use streamweave_distributed::{WorkerId, WorkerState};

proptest! {
  #[test]
  fn test_coordinator_config_default(_ in any::<u8>()) {
    let config = CoordinatorConfig::default();
    prop_assert_eq!(config.heartbeat_timeout, Duration::from_secs(30));
    prop_assert_eq!(config.max_workers, 100);
    prop_assert_eq!(config.rebalance_strategy, RebalanceStrategy::Gradual);
  }

  #[test]
  fn test_worker_info_capacity(
    capacity in 1usize..1000,
    current_load in 0usize..1000,
  ) {
    let info = WorkerInfo {
      worker_id: WorkerId::new(format!("worker-{}", capacity)),
      state: WorkerState::Ready,
      address: "127.0.0.1:8080".parse().unwrap(),
      last_heartbeat: None,
      assigned_partitions: vec![],
      capacity,
      current_load,
    };

    // Worker has capacity if current_load < capacity
    prop_assert_eq!(info.has_capacity(), current_load < capacity);
  }

  #[test]
  fn test_worker_info_health_with_recent_heartbeat(
    timeout_secs in 1u64..3600,
    seconds_ago in 0u64..60,
  ) {
    let timeout = Duration::from_secs(timeout_secs);
    let last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(seconds_ago as i64);

    let info = WorkerInfo {
      worker_id: WorkerId::new("worker-1".to_string()),
      state: WorkerState::Ready,
      address: "127.0.0.1:8080".parse().unwrap(),
      last_heartbeat: Some(last_heartbeat),
      assigned_partitions: vec![],
      capacity: 10,
      current_load: 0,
    };

    // Worker is healthy if seconds_ago < timeout_secs
    prop_assert_eq!(info.is_healthy(timeout), seconds_ago < timeout_secs);
  }

  #[test]
  fn test_worker_info_health_no_heartbeat(_timeout_secs in 1u64..3600) {
    let timeout = Duration::from_secs(_timeout_secs);

    let info = WorkerInfo {
      worker_id: WorkerId::new("worker-1".to_string()),
      state: WorkerState::Ready,
      address: "127.0.0.1:8080".parse().unwrap(),
      last_heartbeat: None,
      assigned_partitions: vec![],
      capacity: 10,
      current_load: 0,
    };

    // Worker without heartbeat is never healthy
    prop_assert!(!info.is_healthy(timeout));
  }
}

#[test]
fn test_rebalance_strategy_default() {
  assert_eq!(RebalanceStrategy::default(), RebalanceStrategy::Gradual);
}

#[test]
fn test_rebalance_strategy_serialization() {
  let strategies = vec![
    RebalanceStrategy::Immediate,
    RebalanceStrategy::Gradual,
    RebalanceStrategy::Manual,
  ];

  for strategy in strategies {
    let serialized = serde_json::to_string(&strategy).unwrap();
    let deserialized: RebalanceStrategy = serde_json::from_str(&serialized).unwrap();
    assert_eq!(strategy, deserialized);
  }
}

#[test]
fn test_coordinator_error_display() {
  let error = CoordinatorError::RegistrationFailed("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Registration failed"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_coordinator_error_worker_not_found() {
  let worker_id = WorkerId::new("worker-1".to_string());
  let error = CoordinatorError::WorkerNotFound(worker_id.clone());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Worker not found"));
  assert!(error_str.contains("worker-1"));
}

#[test]
fn test_coordinator_error_communication() {
  let error = CoordinatorError::Communication("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Communication error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_coordinator_error_invalid_partition() {
  let error = CoordinatorError::InvalidPartition("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Invalid partition"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_coordinator_error_rebalance_failed() {
  let error = CoordinatorError::RebalanceFailed("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Rebalance failed"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_coordinator_error_state_error() {
  let error = CoordinatorError::StateError("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("State error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_coordinator_error_other() {
  let error = CoordinatorError::Other("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Coordinator error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_coordinator_error_debug() {
  let error = CoordinatorError::Other("test".to_string());
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_coordinator_error_is_error() {
  let error = CoordinatorError::Other("test".to_string());
  use std::error::Error;
  assert!(!error.source().is_some());
}

#[test]
fn test_worker_info_serialization() {
  let info = WorkerInfo {
    worker_id: WorkerId::new("worker-1".to_string()),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now()),
    assigned_partitions: vec![0, 1, 2],
    capacity: 10,
    current_load: 5,
  };

  let serialized = serde_json::to_string(&info).unwrap();
  let deserialized: WorkerInfo = serde_json::from_str(&serialized).unwrap();

  assert_eq!(info.worker_id, deserialized.worker_id);
  assert_eq!(info.state, deserialized.state);
  assert_eq!(info.assigned_partitions, deserialized.assigned_partitions);
  assert_eq!(info.capacity, deserialized.capacity);
  assert_eq!(info.current_load, deserialized.current_load);
}

#[test]
fn test_worker_info_clone() {
  let info1 = WorkerInfo {
    worker_id: WorkerId::new("worker-1".to_string()),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now()),
    assigned_partitions: vec![0, 1],
    capacity: 10,
    current_load: 5,
  };

  let info2 = info1.clone();
  assert_eq!(info1.worker_id, info2.worker_id);
  assert_eq!(info1.state, info2.state);
  assert_eq!(info1.assigned_partitions, info2.assigned_partitions);
}

#[test]
fn test_coordinator_config_serialization() {
  let config = CoordinatorConfig {
    bind_address: "0.0.0.0:8080".parse().unwrap(),
    heartbeat_timeout: Duration::from_secs(30),
    health_check_interval: Duration::from_secs(10),
    max_workers: 100,
    rebalance_strategy: RebalanceStrategy::Immediate,
    enable_failover: true,
  };

  let serialized = serde_json::to_string(&config).unwrap();
  let deserialized: CoordinatorConfig = serde_json::from_str(&serialized).unwrap();

  assert_eq!(config.max_workers, deserialized.max_workers);
  assert_eq!(config.rebalance_strategy, deserialized.rebalance_strategy);
  assert_eq!(config.enable_failover, deserialized.enable_failover);
}
