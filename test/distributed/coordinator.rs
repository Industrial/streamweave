//! Tests for distributed coordinator module

use std::net::SocketAddr;
use std::time::Duration;
use streamweave::distributed::coordinator::{CoordinatorConfig, RebalanceStrategy, WorkerInfo};
use streamweave::distributed::worker::{WorkerId, WorkerState};

#[test]
fn test_rebalance_strategy_default() {
  let strategy = RebalanceStrategy::default();
  assert_eq!(strategy, RebalanceStrategy::Gradual);
}

#[test]
fn test_rebalance_strategy_variants() {
  assert_eq!(RebalanceStrategy::Immediate, RebalanceStrategy::Immediate);
  assert_eq!(RebalanceStrategy::Gradual, RebalanceStrategy::Gradual);
  assert_eq!(RebalanceStrategy::Manual, RebalanceStrategy::Manual);
}

#[test]
fn test_coordinator_config_default() {
  let config = CoordinatorConfig::default();
  assert_eq!(config.heartbeat_timeout, Duration::from_secs(30));
  assert_eq!(config.health_check_interval, Duration::from_secs(10));
  assert_eq!(config.max_workers, 100);
  assert_eq!(config.rebalance_strategy, RebalanceStrategy::Gradual);
  assert!(!config.enable_failover);
}

#[test]
fn test_worker_info_is_healthy() {
  let mut info = WorkerInfo {
    worker_id: WorkerId::new("worker-1".to_string()),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: Some(chrono::Utc::now()),
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 0,
  };

  // Should be healthy with recent heartbeat
  assert!(info.is_healthy(Duration::from_secs(30)));

  // Should not be healthy with old heartbeat
  info.last_heartbeat = Some(chrono::Utc::now() - chrono::Duration::seconds(60));
  assert!(!info.is_healthy(Duration::from_secs(30)));

  // Should not be healthy with no heartbeat
  info.last_heartbeat = None;
  assert!(!info.is_healthy(Duration::from_secs(30)));
}

#[test]
fn test_worker_info_has_capacity() {
  let mut info = WorkerInfo {
    worker_id: WorkerId::new("worker-1".to_string()),
    state: WorkerState::Ready,
    address: "127.0.0.1:8080".parse().unwrap(),
    last_heartbeat: None,
    assigned_partitions: vec![],
    capacity: 10,
    current_load: 5,
  };

  // Should have capacity
  assert!(info.has_capacity());

  // Should not have capacity when at limit
  info.current_load = 10;
  assert!(!info.has_capacity());

  // Should not have capacity when over limit
  info.current_load = 15;
  assert!(!info.has_capacity());
}
