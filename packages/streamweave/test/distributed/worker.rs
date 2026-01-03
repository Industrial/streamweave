//! Tests for distributed worker module

use std::net::SocketAddr;
use std::time::Duration;
use streamweave::distributed::worker::{WorkerConfig, WorkerError, WorkerId, WorkerState};

#[test]
fn test_worker_id_new() {
  let id = WorkerId::new("worker-1".to_string());
  assert_eq!(id.as_str(), "worker-1");
}

#[test]
fn test_worker_id_from_string() {
  let id: WorkerId = "worker-2".to_string().into();
  assert_eq!(id.as_str(), "worker-2");
}

#[test]
fn test_worker_id_display() {
  let id = WorkerId::new("worker-1".to_string());
  assert_eq!(format!("{}", id), "worker-1");
}

#[test]
fn test_worker_state_default() {
  let state = WorkerState::default();
  assert_eq!(state, WorkerState::Initializing);
}

#[test]
fn test_worker_state_variants() {
  assert_eq!(WorkerState::Initializing, WorkerState::Initializing);
  assert_eq!(WorkerState::Ready, WorkerState::Ready);
  assert_eq!(WorkerState::Processing, WorkerState::Processing);
  assert_eq!(WorkerState::Error, WorkerState::Error);
  assert_eq!(WorkerState::ShuttingDown, WorkerState::ShuttingDown);
  assert_eq!(WorkerState::Terminated, WorkerState::Terminated);
}

#[test]
fn test_worker_config_default() {
  let config = WorkerConfig::default();
  assert_eq!(config.heartbeat_interval, Duration::from_secs(5));
  assert_eq!(config.communication_timeout, Duration::from_secs(30));
  assert_eq!(config.max_concurrent_tasks, 10);
  assert!(config.assigned_partitions.is_empty());
}

#[test]
fn test_worker_error_display() {
  let err = WorkerError::RegistrationFailed("test".to_string());
  assert!(err.to_string().contains("Registration failed"));

  let err = WorkerError::Communication("test".to_string());
  assert!(err.to_string().contains("Communication"));

  let err = WorkerError::InvalidTask("test".to_string());
  assert!(err.to_string().contains("Invalid task"));

  let err = WorkerError::PipelineError("test".to_string());
  assert!(err.to_string().contains("Pipeline error"));

  let err = WorkerError::StateError("test".to_string());
  assert!(err.to_string().contains("State error"));

  let err = WorkerError::Other("test".to_string());
  assert!(err.to_string().contains("Worker error"));
}

#[test]
fn test_worker_error_is_error() {
  let err = WorkerError::RegistrationFailed("test".to_string());
  // Should compile - implements Error trait
  let _: &dyn std::error::Error = &err;
  assert!(true);
}
