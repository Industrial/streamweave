//! Tests for distributed pool module

use std::net::SocketAddr;
use std::time::Duration;
use streamweave::distributed::pool::{ConnectionPool, ConnectionPoolConfig, PoolError};

#[test]
fn test_connection_pool_config_default() {
  let config = ConnectionPoolConfig::default();
  assert_eq!(config.max_connections, 100);
  assert_eq!(config.max_idle, 10);
  assert_eq!(config.connect_timeout, Duration::from_secs(30));
  assert_eq!(config.idle_timeout, Duration::from_secs(300));
  assert_eq!(config.max_retries, 3);
  assert_eq!(config.retry_delay, Duration::from_secs(1));
}

#[test]
fn test_connection_pool_new() {
  let config = ConnectionPoolConfig::default();
  let pool = ConnectionPool::new(config);
  // Should create successfully
  assert!(true);
}

#[test]
fn test_pool_error_display() {
  use streamweave::distributed::connection::ConnectionError;

  let err = PoolError::Connection(ConnectionError::Timeout);
  assert!(err.to_string().contains("Connection error"));

  let err = PoolError::PoolExhausted;
  assert!(err.to_string().contains("Connection pool exhausted"));

  let err = PoolError::ConnectionNotFound;
  assert!(err.to_string().contains("Connection not found"));

  let err = PoolError::RetryLimitExceeded;
  assert!(err.to_string().contains("Retry limit exceeded"));

  let err = PoolError::Other("test".to_string());
  assert!(err.to_string().contains("Pool error"));
}

#[test]
fn test_pool_error_is_error() {
  use streamweave::distributed::connection::ConnectionError;
  let err = PoolError::Connection(ConnectionError::Timeout);
  // Should compile - implements Error trait
  let _: &dyn std::error::Error = &err;
  assert!(true);
}
