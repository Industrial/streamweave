//! Tests for distributed transport module

use streamweave::distributed::protocol::{MessageType, ProtocolMessage, ProtocolVersion};
use streamweave::distributed::transport::{TcpStreamTransport, TransportError, TransportMessage};

#[test]
fn test_tcp_stream_transport_new() {
  let transport = TcpStreamTransport::new();
  // Should create successfully
  assert!(true);
}

#[test]
fn test_tcp_stream_transport_default() {
  let transport = TcpStreamTransport::default();
  // Should create successfully
  assert!(true);
}

#[test]
fn test_tcp_stream_transport_next_sequence() {
  let mut transport = TcpStreamTransport::new();
  assert_eq!(transport.next_sequence(), 1);
  assert_eq!(transport.next_sequence(), 2);
  assert_eq!(transport.next_sequence(), 3);
}

#[test]
fn test_transport_message_new() {
  let msg = TransportMessage {
    payload: ProtocolMessage::new(
      MessageType::Heartbeat,
      vec![],
      "source".to_string(),
      "dest".to_string(),
    ),
    sequence: 1,
    metadata: None,
  };

  assert_eq!(msg.sequence, 1);
  assert!(msg.metadata.is_none());
}

#[test]
fn test_transport_error_display() {
  let err = TransportError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
  assert!(err.to_string().contains("Network error"));

  let err = TransportError::Serialization("test".to_string());
  assert!(err.to_string().contains("Serialization"));

  let err = TransportError::StreamEnded;
  assert!(err.to_string().contains("Stream ended"));

  let err = TransportError::Other("test".to_string());
  assert!(err.to_string().contains("Transport error"));
}

#[test]
fn test_transport_error_is_error() {
  let err = TransportError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
  // Should compile - implements Error trait
  let _: &dyn std::error::Error = &err;
  assert!(true);
}
