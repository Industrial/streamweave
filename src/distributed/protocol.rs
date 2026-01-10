use serde::{Deserialize, Serialize};

/// Protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolVersion {
  /// Version 1 of the protocol.
  V1,
}

impl Default for ProtocolVersion {
  fn default() -> Self {
    Self::V1
  }
}

/// Message types for the distributed protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
  /// Heartbeat message.
  Heartbeat,
  /// Task assignment message.
  TaskAssignment,
  /// Task completion message.
  TaskComplete,
  /// Task failure message.
  TaskFailure,
  /// Checkpoint request message.
  CheckpointRequest,
  /// Checkpoint response message.
  CheckpointResponse,
  /// Rebalance request message.
  RebalanceRequest,
  /// Rebalance acknowledgment message.
  RebalanceAck,
  /// Worker registration message.
  WorkerRegister,
  /// Worker registered confirmation message.
  WorkerRegistered,
  /// Partition data message.
  PartitionData,
}

/// Protocol message for distributed communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
  /// Message type.
  pub message_type: MessageType,
  /// Message payload (serialized bytes).
  pub payload: Vec<u8>,
  /// Source identifier.
  pub source: String,
  /// Destination identifier.
  pub destination: String,
  /// Protocol version.
  pub version: ProtocolVersion,
  /// Optional correlation ID for request/response matching.
  pub correlation_id: Option<String>,
}

impl ProtocolMessage {
  /// Creates a new protocol message.
  ///
  /// # Arguments
  ///
  /// * `message_type` - The type of message.
  /// * `payload` - The message payload.
  /// * `source` - The source identifier.
  /// * `destination` - The destination identifier.
  ///
  /// # Returns
  ///
  /// A new `ProtocolMessage` instance.
  pub fn new(
    message_type: MessageType,
    payload: Vec<u8>,
    source: String,
    destination: String,
  ) -> Self {
    Self {
      message_type,
      payload,
      source,
      destination,
      version: ProtocolVersion::default(),
      correlation_id: None,
    }
  }

  /// Sets the correlation ID for request/response matching.
  ///
  /// # Arguments
  ///
  /// * `id` - The correlation ID.
  ///
  /// # Returns
  ///
  /// `self` for method chaining.
  pub fn with_correlation_id(mut self, id: String) -> Self {
    self.correlation_id = Some(id);
    self
  }
}

/// Protocol-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
  /// Protocol timeout.
  #[error("Protocol timeout")]
  Timeout,
  /// Invalid message format.
  #[error("Invalid message format: {0}")]
  InvalidFormat(String),
  /// Unsupported protocol version.
  #[error("Unsupported protocol version")]
  UnsupportedVersion,
  /// Serialization error.
  #[error("Serialization error: {0}")]
  Serialization(String),
  /// Deserialization error.
  #[error("Deserialization error: {0}")]
  Deserialization(String),
  /// Other protocol error.
  #[error("Protocol error: {0}")]
  Other(String),
}
