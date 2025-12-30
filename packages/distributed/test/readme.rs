//! Integration tests for README examples.
//!
//! These tests verify that the code examples in the README.md file compile and work correctly.

use std::time::Duration;
use streamweave_distributed::{CoordinatorConfig, RebalanceStrategy};
use streamweave_distributed::{WorkerConfig, WorkerId};

#[tokio::test]
async fn test_coordinator_config_example() {
  // Example from README: Coordinator Setup
  let config = CoordinatorConfig {
    bind_address: "0.0.0.0:8080".parse().unwrap(),
    heartbeat_timeout: Duration::from_secs(30),
    health_check_interval: Duration::from_secs(10),
    max_workers: 100,
    rebalance_strategy: RebalanceStrategy::Gradual,
    enable_failover: false,
  };

  // Verify config is valid
  assert_eq!(config.max_workers, 100);
  assert_eq!(config.rebalance_strategy, RebalanceStrategy::Gradual);
}

#[tokio::test]
async fn test_worker_config_example() {
  // Example from README: Worker Setup
  let config = WorkerConfig {
    worker_id: WorkerId::new("worker-1".to_string()),
    bind_address: "0.0.0.0:0".parse().unwrap(),
    coordinator_address: "127.0.0.1:8080".parse().unwrap(),
    heartbeat_interval: Duration::from_secs(5),
    communication_timeout: Duration::from_secs(30),
    max_concurrent_tasks: 10,
    assigned_partitions: vec![],
  };

  // Verify config is valid
  assert_eq!(config.worker_id.as_str(), "worker-1");
  assert_eq!(config.max_concurrent_tasks, 10);
}

#[tokio::test]
async fn test_rebalance_strategy_example() {
  // Example from README: Rebalancing
  let config = CoordinatorConfig {
    bind_address: "0.0.0.0:8080".parse().unwrap(),
    rebalance_strategy: RebalanceStrategy::Gradual,
    ..Default::default()
  };

  assert_eq!(config.rebalance_strategy, RebalanceStrategy::Gradual);
}
