use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Unique identifier for a connection
pub type ConnectionId = Uuid;

/// Unique identifier for a request
pub type RequestId = Uuid;

/// Information about an active connection
#[derive(Debug)]
pub struct ConnectionInfo {
  pub id: ConnectionId,
  pub stream: Arc<Mutex<TcpStream>>,
  pub remote_addr: std::net::SocketAddr,
  pub created_at: std::time::Instant,
}

/// Manages HTTP connections and request-to-connection mapping
#[derive(Debug)]
pub struct ConnectionManager {
  /// Active connections by connection ID
  connections: Arc<RwLock<HashMap<ConnectionId, ConnectionInfo>>>,
  /// Request ID to Connection ID mapping
  request_to_connection: Arc<RwLock<HashMap<RequestId, ConnectionId>>>,
  /// Maximum number of connections
  max_connections: usize,
}

impl ConnectionManager {
  /// Create a new connection manager
  pub fn new(max_connections: usize) -> Self {
    Self {
      connections: Arc::new(RwLock::new(HashMap::new())),
      request_to_connection: Arc::new(RwLock::new(HashMap::new())),
      max_connections,
    }
  }

  /// Register a new connection
  pub async fn register_connection(
    &self,
    stream: TcpStream,
    remote_addr: std::net::SocketAddr,
  ) -> Result<ConnectionId, String> {
    let mut connections = self.connections.write().await;

    if connections.len() >= self.max_connections {
      return Err("Maximum connections reached".to_string());
    }

    let connection_id = Uuid::new_v4();
    let connection_info = ConnectionInfo {
      id: connection_id,
      stream: Arc::new(Mutex::new(stream)),
      remote_addr,
      created_at: std::time::Instant::now(),
    };

    connections.insert(connection_id, connection_info);
    Ok(connection_id)
  }

  /// Register a request and map it to a connection
  pub async fn register_request(&self, request_id: RequestId, connection_id: ConnectionId) {
    let mut mapping = self.request_to_connection.write().await;
    mapping.insert(request_id, connection_id);
  }

  /// Get a connection by ID
  pub async fn get_connection(&self, connection_id: ConnectionId) -> Option<Arc<Mutex<TcpStream>>> {
    let connections = self.connections.read().await;
    connections
      .get(&connection_id)
      .map(|info| info.stream.clone())
  }

  /// Get connection ID for a request
  pub async fn get_connection_for_request(&self, request_id: RequestId) -> Option<ConnectionId> {
    let mapping = self.request_to_connection.read().await;
    mapping.get(&request_id).copied()
  }

  /// Remove a connection
  pub async fn remove_connection(&self, connection_id: ConnectionId) {
    let mut connections = self.connections.write().await;
    connections.remove(&connection_id);
  }

  /// Remove request mapping
  pub async fn remove_request(&self, request_id: RequestId) {
    let mut mapping = self.request_to_connection.write().await;
    mapping.remove(&request_id);
  }

  /// Get connection count
  pub async fn connection_count(&self) -> usize {
    let connections = self.connections.read().await;
    connections.len()
  }

  /// Clean up old connections (for future use with timeouts)
  pub async fn cleanup_old_connections(&self, max_age: std::time::Duration) {
    let mut connections = self.connections.write().await;
    let now = std::time::Instant::now();

    connections.retain(|_, info| now.duration_since(info.created_at) < max_age);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use tokio::net::TcpListener;

  #[tokio::test]
  async fn test_connection_manager_creation() {
    let manager = ConnectionManager::new(100);
    assert_eq!(manager.connection_count().await, 0);
  }

  #[tokio::test]
  async fn test_register_connection() {
    let manager = ConnectionManager::new(100);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Create a mock connection
    let stream = TcpStream::connect(addr).await.unwrap();
    let connection_id = manager.register_connection(stream, addr).await.unwrap();

    assert_eq!(manager.connection_count().await, 1);
    assert!(manager.get_connection(connection_id).await.is_some());
  }

  #[tokio::test]
  async fn test_request_mapping() {
    let manager = ConnectionManager::new(100);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let stream = TcpStream::connect(addr).await.unwrap();
    let connection_id = manager.register_connection(stream, addr).await.unwrap();

    let request_id = Uuid::new_v4();
    manager.register_request(request_id, connection_id).await;

    let mapped_connection = manager.get_connection_for_request(request_id).await;
    assert_eq!(mapped_connection, Some(connection_id));
  }

  #[tokio::test]
  async fn test_max_connections() {
    let manager = ConnectionManager::new(1);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let stream1 = TcpStream::connect(addr).await.unwrap();
    let stream2 = TcpStream::connect(addr).await.unwrap();

    let result1 = manager.register_connection(stream1, addr).await;
    let result2 = manager.register_connection(stream2, addr).await;

    assert!(result1.is_ok());
    assert!(result2.is_err());
    assert_eq!(manager.connection_count().await, 1);
  }
}
