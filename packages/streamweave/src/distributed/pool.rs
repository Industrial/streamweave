//! Connection pooling for efficient network resource management.

use super::connection::{Connection, ConnectionError};
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
