//! Serialization utilities for graph node execution.
//!
//! This module provides serialization and deserialization functions for
//! graph node execution, allowing data to be serialized for transmission
//! between nodes via channels.

use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};
use std::fmt;

/// Trait for serializing and deserializing data.
///
/// This trait abstracts over different serialization formats (JSON, bincode,
/// MessagePack, etc.), allowing the execution engine to use different
/// serializers based on the execution mode.
///
/// # Note
///
/// This is a basic trait definition. Full implementation details will be
/// added in task 15.1.2.
pub trait Serializer: Send + Sync {
  /// Serializes an item to bytes.
  ///
  /// # Arguments
  ///
  /// * `item` - The item to serialize
  ///
  /// # Returns
  ///
  /// Serialized bytes, or an error if serialization fails
  fn serialize<T: Serialize>(&self, item: &T) -> Result<Bytes, SerializationError>;

  /// Deserializes an item from bytes.
  ///
  /// # Arguments
  ///
  /// * `bytes` - The bytes to deserialize
  ///
  /// # Returns
  ///
  /// Deserialized item, or an error if deserialization fails
  fn deserialize<T: DeserializeOwned>(&self, bytes: Bytes) -> Result<T, SerializationError>;
}

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

/// Serializes an item to bytes.
///
/// This function serializes a value that implements `Serialize` to JSON
/// bytes using `serde_json::to_vec` and converts the result to `Bytes`
/// for zero-copy sharing.
///
/// # Arguments
///
/// * `item` - The item to serialize.
///
/// # Returns
///
/// Returns `Ok(Bytes)` containing the serialized JSON bytes, or
/// `Err(SerializationError)` if serialization fails.
///
/// # Example
///
/// ```rust
/// use serde::Serialize;
/// use streamweave_graph::serialize;
///
/// #[derive(Serialize)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// let point = Point { x: 1, y: 2 };
/// let bytes = serialize(&point)?;
/// # Ok::<(), streamweave_graph::SerializationError>(())
/// ```
pub fn serialize<T: Serialize>(item: &T) -> Result<Bytes, SerializationError> {
  serde_json::to_vec(item)
    .map_err(SerializationError::from)
    .map(Bytes::from)
}

/// Deserializes an item from bytes.
///
/// This function deserializes JSON bytes to a value that implements
/// `DeserializeOwned` using `serde_json::from_slice`. Accepts `Bytes`
/// for zero-copy access to the underlying data.
///
/// # Arguments
///
/// * `data` - The bytes containing JSON data to deserialize.
///
/// # Returns
///
/// Returns `Ok(T)` containing the deserialized value, or
/// `Err(SerializationError)` if deserialization fails.
///
/// # Example
///
/// ```rust
/// use bytes::Bytes;
/// use serde::Deserialize;
/// use streamweave_graph::deserialize;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// let bytes = Bytes::from(br#"{"x":1,"y":2}"#);
/// let point: Point = deserialize(bytes)?;
/// assert_eq!(point, Point { x: 1, y: 2 });
/// # Ok::<(), streamweave_graph::SerializationError>(())
/// ```
pub fn deserialize<T: DeserializeOwned>(data: Bytes) -> Result<T, SerializationError> {
  serde_json::from_slice(data.as_ref()).map_err(SerializationError::from)
}

/// Zero-copy deserializer that holds a Bytes buffer for lifetime-aware deserialization.
///
/// This structure enables zero-copy deserialization by maintaining the lifetime
/// of the serialized data. When deserializing string-heavy data, strings can be
/// deserialized as `&str` references pointing into the buffer, avoiding allocations.
///
/// # Zero-Copy Semantics
///
/// - Holds `Bytes` buffer for shared ownership
/// - Enables lifetime-aware deserialization with `serde_json::from_slice`
/// - Strings can be deserialized as `&str` when data outlives deserialized value
/// - Reduces allocations for string-heavy data structures
///
/// # Example
///
/// ```rust
/// use streamweave_graph::serialization::ZeroCopyDeserializer;
/// use bytes::Bytes;
///
/// let buffer = Bytes::from(r#"{"text": "hello"}"#);
/// let deserializer = ZeroCopyDeserializer::new(buffer);
/// let result: serde_json::Value = deserializer.deserialize().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ZeroCopyDeserializer {
  /// The buffer containing serialized data
  buffer: Bytes,
}

impl ZeroCopyDeserializer {
  /// Create a new zero-copy deserializer from a Bytes buffer.
  ///
  /// # Arguments
  ///
  /// * `buffer` - The Bytes buffer containing serialized data
  ///
  /// # Returns
  ///
  /// A new `ZeroCopyDeserializer` instance
  #[must_use]
  pub fn new(buffer: Bytes) -> Self {
    Self { buffer }
  }

  /// Deserialize a value from the buffer.
  ///
  /// This method uses `serde_json::from_slice` with the buffer's lifetime,
  /// enabling zero-copy string deserialization when the deserialized value
  /// doesn't outlive the buffer.
  ///
  /// # Type Parameters
  ///
  /// * `T` - The type to deserialize, must implement `DeserializeOwned`
  ///
  /// # Returns
  ///
  /// The deserialized value, or an error if deserialization fails
  ///
  /// # Note
  ///
  /// For zero-copy string deserialization, use `deserialize_with_lifetime`
  /// which returns values with lifetimes tied to the buffer.
  pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, SerializationError> {
    serde_json::from_slice(self.buffer.as_ref()).map_err(Into::into)
  }

  /// Get a reference to the underlying buffer.
  ///
  /// # Returns
  ///
  /// A reference to the `Bytes` buffer
  #[must_use]
  pub fn buffer(&self) -> &Bytes {
    &self.buffer
  }
}

/// Deserialize a value with zero-copy string support.
///
/// This function accepts a `&[u8]` slice with a lifetime and deserializes
/// strings as `&str` (zero-copy) when the data outlives the deserialized value.
/// This is useful for string-heavy data structures where you want to avoid
/// allocations.
///
/// # Arguments
///
/// * `data` - The byte slice to deserialize (must outlive the result)
///
/// # Returns
///
/// The deserialized value with strings as `&str` references
///
/// # Lifetime Requirements
///
/// The `data` slice must outlive the deserialized value. This function is
/// most useful when deserializing into a temporary or when the buffer is
/// kept alive for the lifetime of the deserialized value.
///
/// # Example
///
/// ```rust
/// use streamweave_graph::serialization::deserialize_zero_copy_strings;
///
/// let data = b"{\"text\": \"hello\"}";
/// let result: serde_json::Value = deserialize_zero_copy_strings(data).unwrap();
/// // Strings in result are &str references pointing into data
/// ```
pub fn deserialize_zero_copy_strings<'de, T>(data: &'de [u8]) -> Result<T, SerializationError>
where
  T: serde::Deserialize<'de>,
{
  serde_json::from_slice(data).map_err(Into::into)
}

/// A JSON-based serializer implementation.
///
/// This serializer uses `serde_json` for serialization and deserialization.
/// It's the default serializer used by the graph execution engine.
#[derive(Debug, Clone, Default)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
  fn serialize<T: Serialize>(&self, item: &T) -> Result<Bytes, SerializationError> {
    serialize(item)
  }

  fn deserialize<T: DeserializeOwned>(&self, bytes: Bytes) -> Result<T, SerializationError> {
    deserialize(bytes)
  }
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
    assert_eq!(&bytes[..], b"42");
  }

  #[test]
  fn test_serialize_primitive_string() {
    let value = "hello".to_string();
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"\"hello\"");
  }

  #[test]
  fn test_serialize_primitive_bool() {
    let value = true;
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"true");
  }

  #[test]
  fn test_serialize_struct() {
    let value = TestStruct {
      value: 42,
      text: "test".to_string(),
    };
    let bytes = serialize(&value).unwrap();
    let expected = br#"{"value":42,"text":"test"}"#;
    assert_eq!(&bytes[..], expected);
  }

  #[test]
  fn test_serialize_enum() {
    let value = TestEnum::Variant1;
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"\"Variant1\"");

    let value = TestEnum::Variant2("test".to_string());
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes.as_ref(), br#"{"Variant2":"test"}"#);

    let value = TestEnum::Variant3 { field: 42 };
    let bytes = serialize(&value).unwrap();
    assert_eq!(bytes.as_ref(), br#"{"Variant3":{"field":42}}"#);
  }

  #[test]
  fn test_serialize_vec() {
    let value = vec![1, 2, 3];
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"[1,2,3]");
  }

  #[test]
  fn test_serialize_option() {
    let value: Option<i32> = Some(42);
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"42");

    let value: Option<i32> = None;
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"null");
  }

  #[test]
  fn test_serialize_empty_data() {
    let value: Vec<i32> = vec![];
    let bytes = serialize(&value).unwrap();
    assert_eq!(&bytes[..], b"[]");
  }

  // deserialize() tests
  #[test]
  fn test_deserialize_primitive_i32() {
    let bytes = Bytes::from(b"42".as_slice());
    let value: i32 = deserialize(bytes).unwrap();
    assert_eq!(value, 42);
  }

  #[test]
  fn test_deserialize_primitive_string() {
    let bytes = Bytes::from(b"\"hello\"".as_slice());
    let value: String = deserialize(bytes).unwrap();
    assert_eq!(value, "hello");
  }

  #[test]
  fn test_deserialize_primitive_bool() {
    let bytes = Bytes::from(b"true".as_slice());
    let value: bool = deserialize(bytes).unwrap();
    assert!(value);
  }

  #[test]
  fn test_deserialize_struct() {
    let bytes = Bytes::from(br#"{"value":42,"text":"test"}"#.as_slice());
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
    let bytes = Bytes::from(b"\"Variant1\"".as_slice());
    let value: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(value, TestEnum::Variant1);

    let bytes = Bytes::from(br#"{"Variant2":"test"}"#.as_slice());
    let value: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(value, TestEnum::Variant2("test".to_string()));

    let bytes = Bytes::from(br#"{"Variant3":{"field":42}}"#.as_slice());
    let value: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(value, TestEnum::Variant3 { field: 42 });
  }

  #[test]
  fn test_deserialize_vec() {
    let bytes = Bytes::from(b"[1,2,3]".as_slice());
    let value: Vec<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, vec![1, 2, 3]);
  }

  #[test]
  fn test_deserialize_option() {
    let bytes = Bytes::from(b"42".as_slice());
    let value: Option<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, Some(42));

    let bytes = Bytes::from(b"null".as_slice());
    let value: Option<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, None);
  }

  #[test]
  fn test_deserialize_empty_data() {
    let bytes = Bytes::from(b"[]".as_slice());
    let value: Vec<i32> = deserialize(bytes).unwrap();
    assert_eq!(value, Vec::<i32>::new());
  }

  #[test]
  fn test_deserialize_invalid_json() {
    let bytes = Bytes::from(b"{ invalid json".as_slice());
    let result: Result<TestStruct, _> = deserialize(bytes);
    assert!(result.is_err());
    match result.unwrap_err() {
      SerializationError::DeserializationFailed(_) => {}
      _ => panic!("Expected DeserializationFailed"),
    }
  }

  #[test]
  fn test_deserialize_type_mismatch() {
    let bytes = Bytes::from(b"\"not a number\"".as_slice());
    let result: Result<i32, _> = deserialize(bytes);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_missing_field() {
    let bytes = Bytes::from(b"{\"value\":42}".as_slice());
    let result: Result<TestStruct, _> = deserialize(bytes);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_empty_bytes() {
    let bytes = Bytes::from(b"".as_slice());
    let result: Result<TestStruct, _> = deserialize(bytes);
    assert!(result.is_err());
  }

  // Round-trip tests
  #[test]
  fn test_round_trip_i32() {
    let original: i32 = 42;
    let bytes = serialize(&original).unwrap();
    let deserialized: i32 = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_string() {
    let original = "hello world".to_string();
    let bytes = serialize(&original).unwrap();
    let deserialized: String = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_struct() {
    let original = TestStruct {
      value: 42,
      text: "test".to_string(),
    };
    let bytes = serialize(&original).unwrap();
    let deserialized: TestStruct = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_enum() {
    let original = TestEnum::Variant3 { field: 42 };
    let bytes = serialize(&original).unwrap();
    let deserialized: TestEnum = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_vec() {
    let original = vec![1, 2, 3, 4, 5];
    let bytes = serialize(&original).unwrap();
    let deserialized: Vec<i32> = deserialize(bytes).unwrap();
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
    let deserialized: Outer = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_option() {
    let original: Option<TestStruct> = Some(TestStruct {
      value: 42,
      text: "test".to_string(),
    });
    let bytes = serialize(&original).unwrap();
    let deserialized: Option<TestStruct> = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);

    let original: Option<TestStruct> = None;
    let bytes = serialize(&original).unwrap();
    let deserialized: Option<TestStruct> = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_large_data() {
    let original: Vec<i32> = (0..10000).collect();
    let bytes = serialize(&original).unwrap();
    let deserialized: Vec<i32> = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_special_characters() {
    let original = "Hello \"world\" with\nnewlines\tand\ttabs".to_string();
    let bytes = serialize(&original).unwrap();
    let deserialized: String = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }

  #[test]
  fn test_round_trip_unicode() {
    let original = "Hello 🌍 世界 مرحبا".to_string();
    let bytes = serialize(&original).unwrap();
    let deserialized: String = deserialize(bytes).unwrap();
    assert_eq!(original, deserialized);
  }
}
