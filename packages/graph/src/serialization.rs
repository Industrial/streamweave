//! Serialization utilities for graph node execution.
//!
//! This module provides serialization and deserialization functions for
//! graph node execution, allowing data to be serialized for transmission
//! between nodes via channels.

use serde::{Serialize, de::DeserializeOwned};
use std::fmt;

/// Error type for serialization operations.
///
/// This enum represents errors that can occur during serialization or
/// deserialization of data in graph node execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationError {
  /// Serialization failed.
  ///
  /// Contains a description of why serialization failed.
  SerializationFailed(String),
  /// Deserialization failed.
  ///
  /// Contains a description of why deserialization failed.
  DeserializationFailed(String),
  /// Invalid data format.
  ///
  /// Contains a description of what was invalid about the data.
  InvalidData(String),
}

impl fmt::Display for SerializationError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SerializationError::SerializationFailed(msg) => {
        write!(f, "Serialization failed: {}", msg)
      }
      SerializationError::DeserializationFailed(msg) => {
        write!(f, "Deserialization failed: {}", msg)
      }
      SerializationError::InvalidData(msg) => {
        write!(f, "Invalid data: {}", msg)
      }
    }
  }
}

impl std::error::Error for SerializationError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    None
  }
}

impl From<serde_json::Error> for SerializationError {
  fn from(err: serde_json::Error) -> Self {
    // Determine if it's a serialization or deserialization error based on error type
    if err.is_io() || err.line() == 0 {
      // Likely a serialization error (IO error or no line number)
      SerializationError::SerializationFailed(err.to_string())
    } else {
      // Likely a deserialization error (has line number)
      SerializationError::DeserializationFailed(err.to_string())
    }
  }
}

/// Serializes an item to a byte vector.
///
/// This function serializes a value that implements `Serialize` to a JSON
/// byte vector using `serde_json::to_vec`.
///
/// # Arguments
///
/// * `item` - The item to serialize.
///
/// # Returns
///
/// Returns `Ok(Vec<u8>)` containing the serialized JSON bytes, or
/// `Err(SerializationError)` if serialization fails.
///
/// # Example
///
/// ```rust
/// use serde::Serialize;
/// use streamweave_graph::serialization::serialize;
///
/// #[derive(Serialize)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// let point = Point { x: 1, y: 2 };
/// let bytes = serialize(&point)?;
/// # Ok::<(), streamweave_graph::serialization::SerializationError>(())
/// ```
pub fn serialize<T: Serialize>(item: &T) -> Result<Vec<u8>, SerializationError> {
  serde_json::to_vec(item).map_err(SerializationError::from)
}

/// Deserializes an item from a byte slice.
///
/// This function deserializes JSON bytes to a value that implements
/// `DeserializeOwned` using `serde_json::from_slice`.
///
/// # Arguments
///
/// * `data` - The byte slice containing JSON data to deserialize.
///
/// # Returns
///
/// Returns `Ok(T)` containing the deserialized value, or
/// `Err(SerializationError)` if deserialization fails.
///
/// # Example
///
/// ```rust
/// use serde::Deserialize;
/// use streamweave_graph::serialization::deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// let bytes = br#"{"x":1,"y":2}"#;
/// let point: Point = deserialize(bytes)?;
/// assert_eq!(point, Point { x: 1, y: 2 });
/// # Ok::<(), streamweave_graph::serialization::SerializationError>(())
/// ```
pub fn deserialize<T: DeserializeOwned>(data: &[u8]) -> Result<T, SerializationError> {
  serde_json::from_slice(data).map_err(SerializationError::from)
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::{Deserialize, Serialize};
  use std::error::Error;

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

  // SerializationError tests
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
  fn test_serialization_error_error_trait() {
    let err = SerializationError::SerializationFailed("test".to_string());
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

  // serialize() tests
  #[test]
  fn test_serialize_primitive_i32() {
    let value: i32 = 42;
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"42");
  }

  #[test]
  fn test_serialize_primitive_string() {
    let value = "hello".to_string();
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"\"hello\"");
  }

  #[test]
  fn test_serialize_primitive_bool() {
    let value = true;
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"true");
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
  fn test_serialize_enum() {
    let value = TestEnum::Variant1;
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"\"Variant1\"");

    let value = TestEnum::Variant2("test".to_string());
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, br#"{"Variant2":"test"}"#);

    let value = TestEnum::Variant3 { field: 42 };
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, br#"{"Variant3":{"field":42}}"#);
  }

  #[test]
  fn test_serialize_vec() {
    let value = vec![1, 2, 3];
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"[1,2,3]");
  }

  #[test]
  fn test_serialize_option() {
    let value: Option<i32> = Some(42);
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"42");

    let value: Option<i32> = None;
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"null");
  }

  #[test]
  fn test_serialize_empty_data() {
    let value: Vec<i32> = vec![];
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes, b"[]");
  }

  // deserialize() tests
  #[test]
  fn test_deserialize_primitive_i32() {
    let bytes = b"42";
    let value: i32 = deserialize(bytes).unwrap();
    assert_eq!(value, 42);
  }

  #[test]
  fn test_deserialize_primitive_string() {
    let bytes = b"\"hello\"";
    let value: String = deserialize(bytes).unwrap();
    assert_eq!(value, "hello");
  }

  #[test]
  fn test_deserialize_primitive_bool() {
    let bytes = b"true";
    let value: bool = deserialize(bytes).unwrap();
    assert!(value);
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
  fn test_deserialize_enum() {
    let bytes = b"\"Variant1\"";
    let value: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(value, TestEnum::Variant1);

    let bytes = br#"{"Variant2":"test"}"#;
    let value: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(value, TestEnum::Variant2("test".to_string()));

    let bytes = br#"{"Variant3":{"field":42}}"#;
    let value: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(value, TestEnum::Variant3 { field: 42 });
  }

  #[test]
  fn test_deserialize_vec() {
    let bytes = b"[1,2,3]";
    let value: Vec<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, vec![1, 2, 3]);
  }

  #[test]
  fn test_deserialize_option() {
    let bytes = b"42";
    let value: Option<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, Some(42));

    let bytes = b"null";
    let value: Option<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, None);
  }

  #[test]
  fn test_deserialize_empty_data() {
    let bytes = b"[]";
    let value: Vec<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, Vec::<i32>::new());
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
  fn test_deserialize_type_mismatch() {
    let bytes = b"\"not a number\"";
    let result: Result<i32, _> = deserialize(bytes);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_missing_field() {
    let bytes = b"{\"value\":42}";
    let result: Result<TestStruct, _> = deserialize(bytes);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_empty_bytes() {
    let bytes = b"";
    let result: Result<TestStruct, _> = deserialize(bytes);
    assert!(result.is_err());
  }

  // Round-trip tests
  #[test]
  fn test_round_trip_i32() {
    let original: i32 = 42;
    let bytes = serialize(&original).unwrap();
    let deserialized: i32 = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_string() {
    let original = "hello world".to_string();
    let bytes = serialize(&original).unwrap();
    let deserialized: String = deserialize(&bytes).unwrap();
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
  fn test_round_trip_enum() {
    let original = TestEnum::Variant3 { field: 42 };
    let bytes = serialize(&original).unwrap();
    let deserialized: TestEnum = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_vec() {
    let original = vec![1, 2, 3, 4, 5];
    let bytes = serialize(&original).unwrap();
    let deserialized: Vec<i32> = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_nested_struct() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Outer {
      inner: TestStruct,
      count: i32,
    }

    let original = Outer {
      inner: TestStruct {
        value: 42,
        text: "test".to_string(),
      },
      count: 10,
    };
    let bytes = serialize(&original).unwrap();
    let deserialized: Outer = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_option() {
    let original: Option<TestStruct> = Some(TestStruct {
      value: 42,
      text: "test".to_string(),
    });
    let bytes = serialize(&original).unwrap();
    let deserialized: Option<TestStruct> = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);

    let original: Option<TestStruct> = None;
    let bytes = serialize(&original).unwrap();
    let deserialized: Option<TestStruct> = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_large_data() {
    let original: Vec<i32> = (0..10000).collect();
    let bytes = serialize(&original).unwrap();
    let deserialized: Vec<i32> = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_special_characters() {
    let original = "Hello \"world\" with\nnewlines\tand\ttabs".to_string();
    let bytes = serialize(&original).unwrap();
    let deserialized: String = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_unicode() {
    let original = "Hello 🌍 世界 مرحبا".to_string();
    let bytes = serialize(&original).unwrap();
    let deserialized: String = deserialize(&bytes).unwrap();
    assert_eq!(original, deserialized);
  }
}
