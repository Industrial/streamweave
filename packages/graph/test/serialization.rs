//! Serialization tests
//!
//! This module provides integration tests for serialization functionality.

use serde::{Deserialize, Serialize};
use streamweave_graph::{SerializationError, deserialize, serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct TestStruct {
  value: i32,
  text: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum TestEnum {
  Variant1,
  Variant2(String),
  Variant3 { field: i32 },
}

#[test]
fn test_serialization_error_display() {
  let err = SerializationError::SerializationFailed("test error".to_string());
  assert_eq!(err.to_string(), "Serialization failed: test error");

  let err = SerializationError::DeserializationFailed("test error".to_string());
  assert_eq!(err.to_string(), "Deserialization failed: test error");

  let err = SerializationError::InvalidData("test error".to_string());
  assert_eq!(err.to_string(), "Invalid data: test error");
}

#[test]
fn test_serialize_primitive_i32() {
  let value: i32 = 42;
  let bytes = serialize(&value).unwrap();
  assert_eq!(bytes, b"42");
}

#[test]
fn test_serialize_struct() {
  let value = TestStruct {
    value: 42,
    text: "test".to_string(),
  };
  let bytes = serialize(&value).unwrap();
  let expected = br#"{"value":42,"text":"test"}"#;
  assert_eq!(bytes, expected);
}

#[test]
fn test_deserialize_primitive_i32() {
  let bytes = b"42";
  let value: i32 = deserialize(bytes).unwrap();
  assert_eq!(value, 42);
}

#[test]
fn test_deserialize_struct() {
  let bytes = br#"{"value":42,"text":"test"}"#;
  let value: TestStruct = deserialize(bytes).unwrap();
  assert_eq!(
    value,
    TestStruct {
      value: 42,
      text: "test".to_string(),
    }
  );
}

#[test]
fn test_round_trip_i32() {
  let original: i32 = 42;
  let bytes = serialize(&original).unwrap();
  let deserialized: i32 = deserialize(&bytes).unwrap();
  assert_eq!(original, deserialized);
}

#[test]
fn test_round_trip_struct() {
  let original = TestStruct {
    value: 42,
    text: "test".to_string(),
  };
  let bytes = serialize(&original).unwrap();
  let deserialized: TestStruct = deserialize(&bytes).unwrap();
  assert_eq!(original, deserialized);
}

#[test]
fn test_deserialize_invalid_json() {
  let bytes = b"{ invalid json";
  let result: Result<TestStruct, _> = deserialize(bytes);
  assert!(result.is_err());
  match result.unwrap_err() {
    SerializationError::DeserializationFailed(_) => {}
    _ => panic!("Expected DeserializationFailed"),
  }
}

#[test]
fn test_serialization_error_clone() {
  let err1 = SerializationError::SerializationFailed("test".to_string());
  let err2 = err1.clone();

  assert_eq!(err1, err2);
}

#[test]
fn test_serialization_error_partial_eq() {
  let err1 = SerializationError::InvalidData("test".to_string());
  let err2 = SerializationError::InvalidData("test".to_string());
  let err3 = SerializationError::InvalidData("different".to_string());

  assert_eq!(err1, err2);
  assert_ne!(err1, err3);
}

#[test]
fn test_serialization_error_debug() {
  let err = SerializationError::SerializationFailed("test".to_string());
  let debug_str = format!("{:?}", err);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_serialization_error_error_trait() {
  use std::error::Error;

  let err = SerializationError::DeserializationFailed("test".to_string());

  // Verify it implements Error trait
  let _error_ref: &dyn Error = &err;
  assert!(err.source().is_none());
}

#[test]
fn test_from_serde_json_error() {
  // Test deserialization error (has line number)
  let invalid_json = b"{ invalid json";
  let serde_err = serde_json::from_slice::<TestStruct>(invalid_json).unwrap_err();
  let our_err: SerializationError = serde_err.into();
  match our_err {
    SerializationError::DeserializationFailed(_) => {}
    _ => panic!("Expected DeserializationFailed"),
  }
}
