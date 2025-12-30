//! Communication protocol for distributed stream processing.
//!
//! Defines message types, protocol versions, and communication patterns
//! between coordinator and worker nodes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Protocol version for backward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProtocolVersion {
  /// Version 1.0 - Initial distributed processing protocol.
  #[default]
  V1,
}

/// Message types for coordinator-worker communication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
  /// Heartbeat from worker to coordinator.
  Heartbeat,
  /// Task assignment from coordinator to worker.
  TaskAssignment,
  /// Task completion notification from worker to coordinator.
  TaskComplete,
  /// Task failure notification from worker to coordinator.
  TaskFailure,
  /// Request for state checkpoint.
  CheckpointRequest,
  /// State checkpoint data.
  CheckpointResponse,
  /// Request to redistribute partitions.
  RebalanceRequest,
  /// Acknowledgment of rebalance.
  RebalanceAck,
  /// Worker registration request.
  WorkerRegister,
  /// Worker registration response.
  WorkerRegistered,
  /// Partition data transfer.
  PartitionData,
}

/// Protocol message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
  /// Unique message identifier.
  pub id: String,
  /// Message type.
  pub message_type: MessageType,
  /// Timestamp when message was created.
  pub timestamp: DateTime<Utc>,
  /// Protocol version.
  pub version: ProtocolVersion,
  /// Payload (serialized message-specific data).
  pub payload: Vec<u8>,
  /// Source node ID.
  pub source: String,
  /// Destination node ID.
  pub destination: String,
  /// Optional correlation ID for request/response pairs.
  pub correlation_id: Option<String>,
}

impl Message {
  /// Creates a new protocol message.
  #[must_use]
  pub fn new(
    message_type: MessageType,
    payload: Vec<u8>,
    source: String,
    destination: String,
  ) -> Self {
    Self {
      id: format!(
        "msg-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
      ),
      message_type,
      timestamp: Utc::now(),
      version: ProtocolVersion::default(),
      payload,
      source,
      destination,
      correlation_id: None,
    }
  }

  /// Sets the correlation ID for request/response matching.
  #[must_use]
  pub fn with_correlation_id(mut self, id: String) -> Self {
    self.correlation_id = Some(id);
    self
  }
}

/// Protocol-related errors.
#[derive(Debug)]
pub enum ProtocolError {
  /// Message serialization failed.
  Serialization(String),
  /// Message deserialization failed.
  Deserialization(String),
  /// Unsupported protocol version.
  UnsupportedVersion(ProtocolVersion),
  /// Invalid message format.
  InvalidMessage(String),
  /// Communication timeout.
  Timeout,
  /// Connection error.
  Connection(String),
  /// Other protocol error.
  Other(String),
}

impl fmt::Display for ProtocolError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ProtocolError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
      ProtocolError::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
      ProtocolError::UnsupportedVersion(v) => {
        write!(f, "Unsupported protocol version: {:?}", v)
      }
      ProtocolError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
      ProtocolError::Timeout => write!(f, "Protocol timeout"),
      ProtocolError::Connection(msg) => write!(f, "Connection error: {}", msg),
      ProtocolError::Other(msg) => write!(f, "Protocol error: {}", msg),
    }
  }
}

impl std::error::Error for ProtocolError {}
