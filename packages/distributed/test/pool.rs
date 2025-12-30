use proptest::prelude::*;
use std::time::Duration;
use streamweave_distributed::{ConnectionPool, ConnectionPoolConfig, PoolError};

async fn test_connection_pool_config_async() {
  let config = ConnectionPoolConfig::default();
  assert_eq!(config.max_connections, 100);
  assert_eq!(config.max_idle, 10);
  assert_eq!(config.connect_timeout, Duration::from_secs(30));
  assert_eq!(config.idle_timeout, Duration::from_secs(300));
  assert_eq!(config.max_retries, 3);
  assert_eq!(config.retry_delay, Duration::from_secs(1));
}

async fn test_connection_pool_creation_async(
  max_connections: usize,
  max_idle: usize,
  connect_timeout_secs: u64,
  idle_timeout_secs: u64,
  max_retries: usize,
  retry_delay_secs: u64,
) {
  let config = ConnectionPoolConfig {
    max_connections,
    max_idle,
    connect_timeout: Duration::from_secs(connect_timeout_secs),
    idle_timeout: Duration::from_secs(idle_timeout_secs),
    max_retries,
    retry_delay: Duration::from_secs(retry_delay_secs),
  };
  let pool = ConnectionPool::new(config.clone());
  assert_eq!(pool.config.max_connections, max_connections);
  assert_eq!(pool.config.max_idle, max_idle);
  assert_eq!(
    pool.config.connect_timeout,
    Duration::from_secs(connect_timeout_secs)
  );
  assert_eq!(
    pool.config.idle_timeout,
    Duration::from_secs(idle_timeout_secs)
  );
  assert_eq!(pool.config.max_retries, max_retries);
  assert_eq!(
    pool.config.retry_delay,
    Duration::from_secs(retry_delay_secs)
  );
}

proptest! {
  #[test]
  fn test_connection_pool_config(_ in any::<u8>()) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_connection_pool_config_async());
  }

  #[test]
  fn test_connection_pool_creation(
    max_connections in 1usize..1000,
    max_idle in 1usize..100,
    connect_timeout_secs in 1u64..3600,
    idle_timeout_secs in 1u64..3600,
    max_retries in 0usize..10,
    retry_delay_secs in 0u64..60,
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_connection_pool_creation_async(
      max_connections,
      max_idle,
      connect_timeout_secs,
      idle_timeout_secs,
      max_retries,
      retry_delay_secs,
    ));
  }
}

#[tokio::test]
async fn test_connection_pool_get_connection() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    let (_stream, _) = listener.accept().await.unwrap();
  });

  let config = ConnectionPoolConfig::default();
  let pool = ConnectionPool::new(config);

  let result = pool.get_connection(addr).await;
  assert!(result.is_ok());
  let conn = result.unwrap();
  assert!(conn.is_connected());
}

#[tokio::test]
async fn test_connection_pool_return_connection() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    let (_stream, _) = listener.accept().await.unwrap();
  });

  let config = ConnectionPoolConfig {
    max_idle: 5,
    ..Default::default()
  };
  let pool = ConnectionPool::new(config);

  // Get connection
  let conn = pool.get_connection(addr).await.unwrap();

  // Return connection
  pool.return_connection(conn, addr).await;

  // Get connection again (should reuse)
  let conn2 = pool.get_connection(addr).await.unwrap();
  assert!(conn2.is_connected());
}

#[tokio::test]
async fn test_connection_pool_close_all() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    let (_stream, _) = listener.accept().await.unwrap();
  });

  let pool = ConnectionPool::new(ConnectionPoolConfig::default());
  let _conn = pool.get_connection(addr).await.unwrap();

  pool.close_all().await.unwrap();
}

#[tokio::test]
async fn test_connection_pool_cleanup_idle() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    let (_stream, _) = listener.accept().await.unwrap();
  });

  let pool = ConnectionPool::new(ConnectionPoolConfig::default());
  let conn = pool.get_connection(addr).await.unwrap();
  pool.return_connection(conn, addr).await;

  pool.cleanup_idle().await;
}

#[tokio::test]
async fn test_connection_pool_max_idle() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    loop {
      let (_stream, _) = listener.accept().await.unwrap();
    }
  });

  let config = ConnectionPoolConfig {
    max_idle: 2,
    ..Default::default()
  };
  let pool = ConnectionPool::new(config);

  // Return more connections than max_idle
  for _ in 0..5 {
    let conn = pool.get_connection(addr).await.unwrap();
    pool.return_connection(conn, addr).await;
  }

  // Should still work (excess connections are closed)
  let conn = pool.get_connection(addr).await.unwrap();
  assert!(conn.is_connected());
}

#[tokio::test]
async fn test_connection_pool_return_closed_connection() {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    let (_stream, _) = listener.accept().await.unwrap();
  });

  let pool = ConnectionPool::new(ConnectionPoolConfig::default());
  let mut conn = pool.get_connection(addr).await.unwrap();
  conn.close().await.unwrap();

  // Returning a closed connection should not panic
  pool.return_connection(conn, addr).await;
}

#[test]
fn test_pool_error_display() {
  let error = PoolError::PoolExhausted;
  let error_str = format!("{}", error);
  assert!(error_str.contains("Connection pool exhausted"));
}

#[test]
fn test_pool_error_connection_not_found() {
  let error = PoolError::ConnectionNotFound;
  let error_str = format!("{}", error);
  assert!(error_str.contains("Connection not found"));
}

#[test]
fn test_pool_error_retry_limit_exceeded() {
  let error = PoolError::RetryLimitExceeded;
  let error_str = format!("{}", error);
  assert!(error_str.contains("Retry limit exceeded"));
}

#[test]
fn test_pool_error_other() {
  let error = PoolError::Other("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Pool error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_pool_error_debug() {
  let error = PoolError::PoolExhausted;
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_pool_error_is_error() {
  let error = PoolError::PoolExhausted;
  use std::error::Error;
  assert!(!error.source().is_some());
}

#[test]
fn test_connection_pool_default() {
  let pool = ConnectionPool::default();
  // Should create successfully
  assert!(true); // Just verify it doesn't panic
}
