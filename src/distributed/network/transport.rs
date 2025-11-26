//! Stream transport for efficient data transfer between nodes.

use crate::distributed::protocol::Message;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};

/// Transport message wrapper for streaming data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMessage<T> {
  /// Message payload.
  pub payload: T,
  /// Sequence number for ordering.
  pub sequence: u64,
  /// Metadata.
  pub metadata: Option<serde_json::Value>,
}

/// Transport-related errors.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
  /// Network I/O error.
  #[error("Network error: {0}")]
  Io(#[from] std::io::Error),

  /// Serialization error.
  #[error("Serialization error: {0}")]
  Serialization(String),

  /// Stream ended unexpectedly.
  #[error("Stream ended unexpectedly")]
  StreamEnded,

  /// Other transport error.
  #[error("Transport error: {0}")]
  Other(String),
}

/// Simple stream transport over TCP for sending protocol messages.
pub struct TcpStreamTransport {
  sequence: u64,
}

impl TcpStreamTransport {
  /// Creates a new TCP stream transport.
  pub fn new() -> Self {
    Self { sequence: 0 }
  }

  /// Sends a protocol message over a connection.
  pub async fn send_message<W>(writer: &mut W, message: &Message) -> Result<(), TransportError>
  where
    W: AsyncWrite + Unpin + Send,
  {
    use tokio::io::AsyncWriteExt;

    // Serialize message
    let serialized =
      serde_json::to_vec(message).map_err(|e| TransportError::Serialization(e.to_string()))?;

    // Send length prefix (u32)
    writer.write_u32(serialized.len() as u32).await?;

    // Send message data
    writer.write_all(&serialized).await?;
    writer.flush().await?;

    Ok(())
  }

  /// Receives a protocol message from a connection.
  pub async fn receive_message<R>(reader: &mut R) -> Result<Message, TransportError>
  where
    R: AsyncRead + Unpin + Send,
  {
    use tokio::io::AsyncReadExt;

    // Read length prefix
    let length = reader.read_u32().await? as usize;

    // Read message data
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer).await?;

    // Deserialize message
    let message: Message =
      serde_json::from_slice(&buffer).map_err(|e| TransportError::Serialization(e.to_string()))?;

    Ok(message)
  }

  /// Increments and returns the next sequence number.
  pub fn next_sequence(&mut self) -> u64 {
    self.sequence += 1;
    self.sequence
  }
}

impl Default for TcpStreamTransport {
  fn default() -> Self {
    Self::new()
  }
}
