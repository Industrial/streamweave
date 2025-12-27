//! Connection management for network communication.

use crate::protocol::{Message, ProtocolError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

/// Connection-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
  /// Network I/O error.
  #[error("Network error: {0}")]
  Io(#[from] std::io::Error),

  /// Protocol error during communication.
  #[error("Protocol error: {0}")]
  Protocol(#[from] ProtocolError),

  /// Connection timeout.
  #[error("Connection timeout")]
  Timeout,

  /// Connection closed unexpectedly.
  #[error("Connection closed")]
  Closed,

  /// Invalid address.
  #[error("Invalid address: {0}")]
  InvalidAddress(String),

  /// Serialization/deserialization error.
  #[error("Serialization error: {0}")]
  Serialization(String),

  /// Other connection error.
  #[error("Connection error: {0}")]
  Other(String),
}

/// Connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
  /// Connection is being established.
  Connecting,
  /// Connection is active and ready.
  Connected,
  /// Connection is closing.
  Closing,
  /// Connection is closed.
  Closed,
  /// Connection is in error state.
  Error,
}

/// A network connection for distributed processing.
pub struct Connection {
  /// Remote address.
  pub remote_addr: SocketAddr,
  /// Connection state.
  pub state: ConnectionState,
  /// Underlying TCP stream.
  stream: Option<TcpStream>,
  /// Connection timeout.
  timeout: Duration,
}

impl Connection {
  /// Creates a new outbound connection to the given address.
  pub async fn connect(
    addr: SocketAddr,
    timeout_duration: Duration,
  ) -> Result<Self, ConnectionError> {
    let connection_future = TcpStream::connect(addr);
    let stream = timeout(timeout_duration, connection_future)
      .await
      .map_err(|_| ConnectionError::Timeout)?
      .map_err(ConnectionError::Io)?;

    Ok(Self {
      remote_addr: addr,
      state: ConnectionState::Connected,
      stream: Some(stream),
      timeout: timeout_duration,
    })
  }

  /// Creates a connection from an accepted TCP stream.
  pub fn from_stream(stream: TcpStream, remote_addr: SocketAddr) -> Self {
    Self {
      remote_addr,
      state: ConnectionState::Connected,
      stream: Some(stream),
      timeout: Duration::from_secs(30),
    }
  }

  /// Sends a protocol message over this connection.
  pub async fn send(&mut self, message: &Message) -> Result<(), ConnectionError> {
    let stream = self.stream.as_mut().ok_or(ConnectionError::Closed)?;

    // Serialize message to JSON
    let serialized =
      serde_json::to_vec(message).map_err(|e| ConnectionError::Serialization(e.to_string()))?;

    // Send length prefix
    let length = serialized.len() as u32;
    stream
      .write_u32(length)
      .await
      .map_err(ConnectionError::Io)?;

    // Send message data
    stream
      .write_all(&serialized)
      .await
      .map_err(ConnectionError::Io)?;
    stream.flush().await.map_err(ConnectionError::Io)?;

    Ok(())
  }

  /// Receives a protocol message from this connection.
  pub async fn receive(&mut self) -> Result<Message, ConnectionError> {
    let stream = self.stream.as_mut().ok_or(ConnectionError::Closed)?;

    // Read length prefix
    let length = timeout(self.timeout, stream.read_u32())
      .await
      .map_err(|_| ConnectionError::Timeout)?
      .map_err(ConnectionError::Io)? as usize;

    // Read message data
    let mut buffer = vec![0u8; length];
    timeout(self.timeout, stream.read_exact(&mut buffer))
      .await
      .map_err(|_| ConnectionError::Timeout)?
      .map_err(ConnectionError::Io)?;

    // Deserialize message
    let message: Message =
      serde_json::from_slice(&buffer).map_err(|e| ConnectionError::Serialization(e.to_string()))?;

    Ok(message)
  }

  /// Closes the connection gracefully.
  pub async fn close(&mut self) -> Result<(), ConnectionError> {
    self.state = ConnectionState::Closing;
    if let Some(mut stream) = self.stream.take() {
      let _ = stream.shutdown().await;
    }
    self.state = ConnectionState::Closed;
    Ok(())
  }

  /// Checks if the connection is active.
  #[must_use]
  pub fn is_connected(&self) -> bool {
    matches!(self.state, ConnectionState::Connected)
  }
}

/// Trait for managing connections.
#[async_trait::async_trait]
pub trait ConnectionManager: Send + Sync {
  /// Establishes a connection to the given address.
  async fn connect(&mut self, addr: SocketAddr) -> Result<Connection, ConnectionError>;

  /// Accepts an incoming connection.
  async fn accept(&mut self) -> Result<Connection, ConnectionError>;

  /// Closes all connections.
  async fn close_all(&mut self) -> Result<(), ConnectionError>;
}

/// Simple connection manager implementation.
pub struct SimpleConnectionManager {
  listener: Option<TcpListener>,
  bind_addr: Option<SocketAddr>,
}

impl SimpleConnectionManager {
  /// Creates a new connection manager.
  #[must_use]
  pub fn new() -> Self {
    Self {
      listener: None,
      bind_addr: None,
    }
  }

  /// Binds to the given address for accepting connections.
  pub async fn bind(&mut self, addr: SocketAddr) -> Result<(), ConnectionError> {
    let listener = TcpListener::bind(addr).await.map_err(ConnectionError::Io)?;
    self.bind_addr = Some(addr);
    self.listener = Some(listener);
    Ok(())
  }
}

impl Default for SimpleConnectionManager {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait::async_trait]
impl ConnectionManager for SimpleConnectionManager {
  async fn connect(&mut self, addr: SocketAddr) -> Result<Connection, ConnectionError> {
    Connection::connect(addr, Duration::from_secs(30)).await
  }

  async fn accept(&mut self) -> Result<Connection, ConnectionError> {
    let listener = self.listener.as_ref().ok_or_else(|| {
      ConnectionError::Other("Listener not bound. Call bind() first.".to_string())
    })?;

    let (stream, addr) = listener.accept().await.map_err(ConnectionError::Io)?;
    Ok(Connection::from_stream(stream, addr))
  }

  async fn close_all(&mut self) -> Result<(), ConnectionError> {
    // Simple implementation - just clear the listener
    self.listener = None;
    self.bind_addr = None;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use tokio::runtime::Runtime;

  async fn test_connection_manager_bind_async() {
    let mut manager = SimpleConnectionManager::new();
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Should bind successfully to port 0 (OS assigns available port)
    let result = manager.bind(addr).await;
    assert!(result.is_ok());
  }

  proptest! {
    #[test]
    fn test_connection_manager_bind(_ in any::<u8>()) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_connection_manager_bind_async());
    }
  }
}
