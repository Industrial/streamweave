use proptest::prelude::*;
use streamweave_distributed::{Message, MessageType, ProtocolError, ProtocolVersion};

#[test]
fn test_message_creation() {
  let msg = Message::new(
    MessageType::Heartbeat,
    vec![1, 2, 3],
    "worker-1".to_string(),
    "coordinator".to_string(),
  );
  assert_eq!(msg.message_type, MessageType::Heartbeat);
  assert_eq!(msg.source, "worker-1");
  assert_eq!(msg.destination, "coordinator");
  assert_eq!(msg.version, ProtocolVersion::V1);
  assert!(!msg.id.is_empty());
  assert!(msg.correlation_id.is_none());
}

#[test]
fn test_message_correlation() {
  let msg = Message::new(
    MessageType::TaskAssignment,
    vec![],
    "coordinator".to_string(),
    "worker-1".to_string(),
  );
  let corr_id = "corr-123".to_string();
  let msg_with_corr = msg.with_correlation_id(corr_id.clone());
  assert_eq!(msg_with_corr.correlation_id, Some(corr_id));
}

#[test]
fn test_protocol_version_default() {
  assert_eq!(ProtocolVersion::default(), ProtocolVersion::V1);
}

#[test]
fn test_protocol_version_serialization() {
  let version = ProtocolVersion::V1;
  let serialized = serde_json::to_string(&version).unwrap();
  let deserialized: ProtocolVersion = serde_json::from_str(&serialized).unwrap();
  assert_eq!(version, deserialized);
}

#[test]
fn test_all_message_types() {
  let types = vec![
    MessageType::Heartbeat,
    MessageType::TaskAssignment,
    MessageType::TaskComplete,
    MessageType::TaskFailure,
    MessageType::CheckpointRequest,
    MessageType::CheckpointResponse,
    MessageType::RebalanceRequest,
    MessageType::RebalanceAck,
    MessageType::WorkerRegister,
    MessageType::WorkerRegistered,
    MessageType::PartitionData,
  ];

  for msg_type in types {
    let msg = Message::new(
      msg_type.clone(),
      vec![],
      "source".to_string(),
      "dest".to_string(),
    );
    assert_eq!(msg.message_type, msg_type);
  }
}

#[test]
fn test_message_type_serialization() {
  let types = vec![
    MessageType::Heartbeat,
    MessageType::TaskAssignment,
    MessageType::TaskComplete,
  ];

  for msg_type in types {
    let serialized = serde_json::to_string(&msg_type).unwrap();
    let deserialized: MessageType = serde_json::from_str(&serialized).unwrap();
    assert_eq!(msg_type, deserialized);
  }
}

#[test]
fn test_message_serialization() {
  let msg = Message::new(
    MessageType::Heartbeat,
    b"payload".to_vec(),
    "source".to_string(),
    "dest".to_string(),
  )
  .with_correlation_id("corr-123".to_string());

  let serialized = serde_json::to_string(&msg).unwrap();
  let deserialized: Message = serde_json::from_str(&serialized).unwrap();

  assert_eq!(msg.message_type, deserialized.message_type);
  assert_eq!(msg.payload, deserialized.payload);
  assert_eq!(msg.source, deserialized.source);
  assert_eq!(msg.destination, deserialized.destination);
  assert_eq!(msg.correlation_id, deserialized.correlation_id);
  assert_eq!(msg.version, deserialized.version);
}

#[test]
fn test_message_clone() {
  let msg1 = Message::new(
    MessageType::Heartbeat,
    vec![1, 2, 3],
    "source".to_string(),
    "dest".to_string(),
  );
  let msg2 = msg1.clone();
  assert_eq!(msg1.message_type, msg2.message_type);
  assert_eq!(msg1.payload, msg2.payload);
}

proptest! {
  #[test]
  fn test_message_with_various_payloads(
    payload in prop::collection::vec(any::<u8>(), 0..1000),
    source in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
    dest in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
  ) {
    let msg = Message::new(
      MessageType::Heartbeat,
      payload.clone(),
      source.clone(),
      dest.clone(),
    );
    prop_assert_eq!(msg.payload, payload);
    prop_assert_eq!(msg.source, source);
    prop_assert_eq!(msg.destination, dest);
  }

  #[test]
  fn test_message_correlation_id_preserved(
    corr_id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
  ) {
    let msg = Message::new(
      MessageType::TaskAssignment,
      vec![],
      "source".to_string(),
      "dest".to_string(),
    )
    .with_correlation_id(corr_id.clone());
    prop_assert_eq!(msg.correlation_id, Some(corr_id));
  }
}

#[test]
fn test_protocol_error_serialization() {
  let error = ProtocolError::Serialization("test error".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Serialization error"));
  assert!(error_str.contains("test error"));
}

#[test]
fn test_protocol_error_deserialization() {
  let error = ProtocolError::Deserialization("test error".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Deserialization error"));
  assert!(error_str.contains("test error"));
}

#[test]
fn test_protocol_error_unsupported_version() {
  let error = ProtocolError::UnsupportedVersion(ProtocolVersion::V1);
  let error_str = format!("{}", error);
  assert!(error_str.contains("Unsupported protocol version"));
}

#[test]
fn test_protocol_error_invalid_message() {
  let error = ProtocolError::InvalidMessage("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Invalid message"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_protocol_error_timeout() {
  let error = ProtocolError::Timeout;
  let error_str = format!("{}", error);
  assert!(error_str.contains("Protocol timeout"));
}

#[test]
fn test_protocol_error_connection() {
  let error = ProtocolError::Connection("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Connection error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_protocol_error_other() {
  let error = ProtocolError::Other("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Protocol error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_protocol_error_debug() {
  let error = ProtocolError::Timeout;
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_protocol_error_is_error() {
  let error = ProtocolError::Timeout;
  // Verify it implements Error trait
  use std::error::Error;
  assert!(!error.source().is_some());
}
