//! Tests for distributed connection module

use streamweave::distributed::connection::{Connection, ConnectionError, ConnectionState};
use streamweave::distributed::protocol::{MessageType, ProtocolMessage};

#[test]
fn test_connection_state_variants() {
  assert_eq!(ConnectionState::Connecting, ConnectionState::Connecting);
  assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
  assert_eq!(ConnectionState::Closing, ConnectionState::Closing);
  assert_eq!(ConnectionState::Closed, ConnectionState::Closed);
  assert_eq!(ConnectionState::Error, ConnectionState::Error);
}

#[test]
fn test_connection_error_display() {
  let err = ConnectionError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
  assert!(err.to_string().contains("Network error"));

  use streamweave::distributed::protocol::ProtocolError;
  let err = ConnectionError::Protocol(ProtocolError::Timeout);
  assert!(err.to_string().contains("Protocol error"));

  let err = ConnectionError::Timeout;
  assert!(err.to_string().contains("Connection timeout"));

  let err = ConnectionError::Closed;
  assert!(err.to_string().contains("Connection closed"));

  let err = ConnectionError::InvalidAddress("test".to_string());
  assert!(err.to_string().contains("Invalid address"));

  let err = ConnectionError::Serialization("test".to_string());
  assert!(err.to_string().contains("Serialization"));

  let err = ConnectionError::Other("test".to_string());
  assert!(err.to_string().contains("Connection error"));
}

#[test]
fn test_connection_error_is_error() {
  let err = ConnectionError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
  // Should compile - implements Error trait
  let _: &dyn std::error::Error = &err;
  assert!(true);
}
