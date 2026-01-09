//! Tests for graph serialization module

use bytes::Bytes;
use streamweave::graph::serialization::{SerializationError, deserialize, serialize};
use streamweave::message::wrap_message;

#[test]
fn test_serialize_deserialize_message() {
  let msg = wrap_message(42i32);
  let bytes: Bytes = serialize(&msg).unwrap();

  let deserialized: streamweave::message::Message<i32> = deserialize(bytes).unwrap();
  assert_eq!(*deserialized.payload(), 42);
}

#[test]
fn test_serialization_error_display() {
  let err = SerializationError::SerializationFailed("test".to_string());
  assert!(err.to_string().contains("Serialization failed"));

  let err = SerializationError::DeserializationFailed("test".to_string());
  assert!(err.to_string().contains("Deserialization failed"));

  let err = SerializationError::InvalidData("test".to_string());
  assert!(err.to_string().contains("Invalid data"));
}

#[test]
fn test_serialization_error_partial_eq() {
  let err1 = SerializationError::SerializationFailed("test".to_string());
  let err2 = SerializationError::SerializationFailed("test".to_string());
  assert_eq!(err1, err2);
}
