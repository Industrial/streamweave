use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Connection state for distributed network connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
  /// Connection is being established.
  Connecting,
  /// Connection is active and ready.
  Connected,
  /// Connection is being closed.
  Closing,
  /// Connection is closed.
  Closed,
  /// Connection is in an error state.
  Error,
}

/// Error type for connection-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
  /// I/O error.
  #[error("Network error: {0}")]
  Io(#[from] io::Error),
  /// Protocol error.
  #[error("Protocol error: {0}")]
  Protocol(#[from] crate::distributed::protocol::ProtocolError),
  /// Connection timeout.
  #[error("Connection timeout")]
  Timeout,
  /// Connection closed.
  #[error("Connection closed")]
  Closed,
  /// Invalid address.
  #[error("Invalid address: {0}")]
  InvalidAddress(String),
  /// Serialization error.
  #[error("Serialization error: {0}")]
  Serialization(String),
  /// Other connection error.
  #[error("Connection error: {0}")]
  Other(String),
}

/// A network connection for distributed processing.
pub struct Connection {
  /// The underlying TCP stream.
  stream: Option<TcpStream>,
  /// Current connection state.
  state: ConnectionState,
  /// Remote address.
  addr: SocketAddr,
}

impl Connection {
  /// Creates a new connection by connecting to the given address.
  ///
  /// # Arguments
  ///
  /// * `addr` - The address to connect to.
  /// * `timeout_duration` - The connection timeout.
  ///
  /// # Returns
  ///
  /// A `Result` containing the connection or an error.
  pub async fn connect(
    addr: SocketAddr,
    timeout_duration: Duration,
  ) -> Result<Self, ConnectionError> {
    let stream = timeout(timeout_duration, TcpStream::connect(addr))
      .await
      .map_err(|_| ConnectionError::Timeout)?
      .map_err(ConnectionError::Io)?;

    Ok(Self {
      stream: Some(stream),
      state: ConnectionState::Connected,
      addr,
    })
  }

  /// Checks if the connection is currently connected.
  ///
  /// # Returns
  ///
  /// `true` if the connection is active, `false` otherwise.
  pub fn is_connected(&self) -> bool {
    self.state == ConnectionState::Connected && self.stream.is_some()
  }

  /// Closes the connection.
  ///
  /// # Returns
  ///
  /// A `Result` indicating success or failure.
  pub async fn close(&mut self) -> Result<(), ConnectionError> {
    self.state = ConnectionState::Closing;
    if let Some(mut stream) = self.stream.take() {
      // Shutdown the write side of the connection
      // The stream will be dropped after this, which closes it
      AsyncWriteExt::shutdown(&mut stream)
        .await
        .map_err(ConnectionError::Io)?;
    }
    self.state = ConnectionState::Closed;
    Ok(())
  }

  /// Returns the remote address.
  ///
  /// # Returns
  ///
  /// The socket address of the remote peer.
  pub fn addr(&self) -> SocketAddr {
    self.addr
  }

  /// Returns the current connection state.
  ///
  /// # Returns
  ///
  /// The current state of the connection.
  pub fn state(&self) -> ConnectionState {
    self.state
  }
}
