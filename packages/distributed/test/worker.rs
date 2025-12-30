use proptest::prelude::*;
use std::time::Duration;
use streamweave_distributed::{WorkerConfig, WorkerError, WorkerId, WorkerState};

#[test]
fn test_worker_id() {
  let id = WorkerId::new("worker-1".to_string());
  assert_eq!(id.as_str(), "worker-1");
  assert_eq!(id.to_string(), "worker-1");
}

#[test]
fn test_worker_id_from_string() {
  let id: WorkerId = "worker-1".to_string().into();
  assert_eq!(id.as_str(), "worker-1");
}

#[test]
fn test_worker_config_default() {
  let config = WorkerConfig::default();
  assert_eq!(config.heartbeat_interval, Duration::from_secs(5));
  assert_eq!(config.max_concurrent_tasks, 10);
  assert!(config.assigned_partitions.is_empty());
}

#[test]
fn test_worker_state_default() {
  assert_eq!(WorkerState::default(), WorkerState::Initializing);
}

#[test]
fn test_worker_state_serialization() {
  let states = vec![
    WorkerState::Initializing,
    WorkerState::Ready,
    WorkerState::Processing,
    WorkerState::Error,
    WorkerState::ShuttingDown,
    WorkerState::Terminated,
  ];

  for state in states {
    let serialized = serde_json::to_string(&state).unwrap();
    let deserialized: WorkerState = serde_json::from_str(&serialized).unwrap();
    assert_eq!(state, deserialized);
  }
}

#[test]
fn test_worker_error_display() {
  let error = WorkerError::RegistrationFailed("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Registration failed"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_worker_error_communication() {
  let error = WorkerError::Communication("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Communication error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_worker_error_invalid_task() {
  let error = WorkerError::InvalidTask("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Invalid task"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_worker_error_pipeline_error() {
  let error = WorkerError::PipelineError("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Pipeline error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_worker_error_state_error() {
  let error = WorkerError::StateError("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("State error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_worker_error_other() {
  let error = WorkerError::Other("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Worker error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_worker_error_debug() {
  let error = WorkerError::Other("test".to_string());
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_worker_error_is_error() {
  let error = WorkerError::Other("test".to_string());
  use std::error::Error;
  assert!(!error.source().is_some());
}

#[test]
fn test_worker_config_serialization() {
  let config = WorkerConfig {
    worker_id: WorkerId::new("worker-1".to_string()),
    bind_address: "0.0.0.0:0".parse().unwrap(),
    coordinator_address: "127.0.0.1:8080".parse().unwrap(),
    heartbeat_interval: Duration::from_secs(5),
    communication_timeout: Duration::from_secs(30),
    max_concurrent_tasks: 10,
    assigned_partitions: vec![0, 1, 2],
  };

  let serialized = serde_json::to_string(&config).unwrap();
  let deserialized: WorkerConfig = serde_json::from_str(&serialized).unwrap();

  assert_eq!(config.worker_id, deserialized.worker_id);
  assert_eq!(
    config.max_concurrent_tasks,
    deserialized.max_concurrent_tasks
  );
  assert_eq!(config.assigned_partitions, deserialized.assigned_partitions);
}

#[test]
fn test_worker_config_clone() {
  let config1 = WorkerConfig {
    worker_id: WorkerId::new("worker-1".to_string()),
    bind_address: "0.0.0.0:0".parse().unwrap(),
    coordinator_address: "127.0.0.1:8080".parse().unwrap(),
    heartbeat_interval: Duration::from_secs(5),
    communication_timeout: Duration::from_secs(30),
    max_concurrent_tasks: 10,
    assigned_partitions: vec![0, 1],
  };

  let config2 = config1.clone();
  assert_eq!(config1.worker_id, config2.worker_id);
  assert_eq!(config1.assigned_partitions, config2.assigned_partitions);
}

proptest! {
  #[test]
  fn test_worker_id_from_various_strings(
    id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
  ) {
    let worker_id: WorkerId = id.clone().into();
    prop_assert_eq!(worker_id.as_str(), id.as_str());
  }
}
