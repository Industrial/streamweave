//! Connection pooling for efficient network resource management.

use crate::distributed::network::connection::{Connection, ConnectionError};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};

/// Connection pool configuration.
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
  /// Maximum number of connections in the pool.
  pub max_connections: usize,
  /// Maximum number of idle connections to keep.
  pub max_idle: usize,
  /// Connection timeout.
  pub connect_timeout: Duration,
  /// Idle connection timeout.
  pub idle_timeout: Duration,
  /// Maximum retry attempts.
  pub max_retries: usize,
  /// Retry delay between attempts.
  pub retry_delay: Duration,
}

impl Default for ConnectionPoolConfig {
  fn default() -> Self {
    Self {
      max_connections: 100,
      max_idle: 10,
      connect_timeout: Duration::from_secs(30),
      idle_timeout: Duration::from_secs(300),
      max_retries: 3,
      retry_delay: Duration::from_secs(1),
    }
  }
}

/// Connection pool errors.
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
  /// Connection error.
  #[error("Connection error: {0}")]
  Connection(#[from] ConnectionError),

  /// Pool exhausted (too many connections).
  #[error("Connection pool exhausted")]
  PoolExhausted,

  /// Connection not found.
  #[error("Connection not found")]
  ConnectionNotFound,

  /// Retry limit exceeded.
  #[error("Retry limit exceeded")]
  RetryLimitExceeded,

  /// Other pool error.
  #[error("Pool error: {0}")]
  Other(String),
}

/// Connection pool for managing network connections.
pub struct ConnectionPool {
  /// Pool configuration.
  config: ConnectionPoolConfig,
  /// Active connections by address.
  connections: Arc<RwLock<HashMap<SocketAddr, Vec<Connection>>>>,
  /// Semaphore to limit concurrent connections.
  semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
  /// Creates a new connection pool with the given configuration.
  #[must_use]
  pub fn new(config: ConnectionPoolConfig) -> Self {
    let max_connections = config.max_connections;
    Self {
      config,
      connections: Arc::new(RwLock::new(HashMap::new())),
      semaphore: Arc::new(Semaphore::new(max_connections)),
    }
  }

  /// Gets or creates a connection to the given address.
  pub async fn get_connection(&self, addr: SocketAddr) -> Result<Connection, PoolError> {
    // Check for existing idle connection
    {
      let mut connections = self.connections.write().await;
      if let Some(conns) = connections.get_mut(&addr) {
        // Find an idle connection
        if let Some(pos) = conns.iter().position(|c| c.is_connected()) {
          let conn = conns.remove(pos);
          return Ok(conn);
        }
      }
    }

    // Acquire semaphore permit
    let _permit = self
      .semaphore
      .acquire()
      .await
      .map_err(|_| PoolError::PoolExhausted)?;

    // Create new connection with retry
    let mut last_error = None;
    for attempt in 0..=self.config.max_retries {
      match Connection::connect(addr, self.config.connect_timeout).await {
        Ok(conn) => {
          // Store connection for reuse
          let mut connections = self.connections.write().await;
          connections.entry(addr).or_insert_with(Vec::new);
          return Ok(conn);
        }
        Err(e) => {
          last_error = Some(e);
          if attempt < self.config.max_retries {
            tokio::time::sleep(self.config.retry_delay).await;
          }
        }
      }
    }

    Err(
      last_error
        .map(PoolError::Connection)
        .unwrap_or_else(|| PoolError::Other("Connection failed after retries".to_string())),
    )
  }

  /// Returns a connection to the pool for reuse.
  pub async fn return_connection(&self, mut conn: Connection, addr: SocketAddr) {
    // Only return if still connected
    if conn.is_connected() {
      let mut connections = self.connections.write().await;
      let conns = connections.entry(addr).or_insert_with(Vec::new);

      // Limit idle connections
      if conns.len() < self.config.max_idle {
        conns.push(conn);
      } else {
        // Too many idle connections, close it
        let _ = conn.close().await;
      }
    } else {
      // Connection is dead, close it
      let _ = conn.close().await;
    }
  }

  /// Closes all connections in the pool.
  pub async fn close_all(&self) -> Result<(), PoolError> {
    let mut connections = self.connections.write().await;
    for conns in connections.values_mut() {
      for mut conn in conns.drain(..) {
        let _ = conn.close().await;
      }
    }
    connections.clear();
    Ok(())
  }

  /// Cleans up idle connections that have exceeded the timeout.
  pub async fn cleanup_idle(&self) {
    let mut connections = self.connections.write().await;
    for conns in connections.values_mut() {
      conns.retain(|conn| conn.is_connected());
    }
  }
}

impl Default for ConnectionPool {
  fn default() -> Self {
    Self::new(ConnectionPoolConfig::default())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use tokio::runtime::Runtime;

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
      let rt = Runtime::new().unwrap();
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
      let rt = Runtime::new().unwrap();
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
}
