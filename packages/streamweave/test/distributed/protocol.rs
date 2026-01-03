//! Tests for distributed protocol module

use streamweave::distributed::protocol::{
  MessageType, ProtocolError, ProtocolMessage, ProtocolVersion,
};

#[test]
fn test_protocol_version_default() {
  let version = ProtocolVersion::default();
  assert_eq!(version, ProtocolVersion::V1);
}

#[test]
fn test_message_type_variants() {
  assert_eq!(MessageType::Heartbeat, MessageType::Heartbeat);
  assert_eq!(MessageType::TaskAssignment, MessageType::TaskAssignment);
  assert_eq!(MessageType::TaskComplete, MessageType::TaskComplete);
  assert_eq!(MessageType::TaskFailure, MessageType::TaskFailure);
  assert_eq!(
    MessageType::CheckpointRequest,
    MessageType::CheckpointRequest
  );
  assert_eq!(
    MessageType::CheckpointResponse,
    MessageType::CheckpointResponse
  );
  assert_eq!(MessageType::RebalanceRequest, MessageType::RebalanceRequest);
  assert_eq!(MessageType::RebalanceAck, MessageType::RebalanceAck);
  assert_eq!(MessageType::WorkerRegister, MessageType::WorkerRegister);
  assert_eq!(MessageType::WorkerRegistered, MessageType::WorkerRegistered);
  assert_eq!(MessageType::PartitionData, MessageType::PartitionData);
}

#[test]
fn test_protocol_message_new() {
  let msg = ProtocolMessage::new(
    MessageType::Heartbeat,
    vec![1, 2, 3],
    "source".to_string(),
    "dest".to_string(),
  );

  assert_eq!(msg.message_type, MessageType::Heartbeat);
  assert_eq!(msg.payload, vec![1, 2, 3]);
  assert_eq!(msg.source, "source");
  assert_eq!(msg.destination, "dest");
  assert_eq!(msg.version, ProtocolVersion::V1);
  assert!(msg.correlation_id.is_none());
}

#[test]
fn test_protocol_message_with_correlation_id() {
  let msg = ProtocolMessage::new(
    MessageType::Heartbeat,
    vec![],
    "source".to_string(),
    "dest".to_string(),
  )
  .with_correlation_id("corr-123".to_string());

  assert_eq!(msg.correlation_id, Some("corr-123".to_string()));
}

#[test]
fn test_protocol_error_display() {
  let err = ProtocolError::Serialization("test".to_string());
  assert!(err.to_string().contains("Serialization"));

  let err = ProtocolError::Deserialization("test".to_string());
  assert!(err.to_string().contains("Deserialization"));

  let err = ProtocolError::UnsupportedVersion(ProtocolVersion::V1);
  assert!(err.to_string().contains("Unsupported"));

  let err = ProtocolError::InvalidMessage("test".to_string());
  assert!(err.to_string().contains("Invalid"));

  let err = ProtocolError::Timeout;
  assert!(err.to_string().contains("timeout"));

  let err = ProtocolError::Connection("test".to_string());
  assert!(err.to_string().contains("Connection"));

  let err = ProtocolError::Other("test".to_string());
  assert!(err.to_string().contains("Protocol error"));
}

#[test]
fn test_protocol_error_is_error() {
  let err = ProtocolError::Serialization("test".to_string());
  // Should compile - implements Error trait
  let _: &dyn std::error::Error = &err;
  assert!(true);
}
