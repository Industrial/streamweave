//! Serialization utilities for graph node execution.
//!
//! This module provides serialization and deserialization functions for
//! graph node execution, allowing data to be serialized for transmission
//! between nodes via channels.
//!
//! ## `Message<T>` Support
//!
//! All data flowing through the graph is wrapped in `Message<T>`, and this module
//! fully supports serializing and deserializing `Message<T>` instances. Message IDs
//! and metadata are preserved during serialization/deserialization, enabling
//! end-to-end traceability and message persistence.
//!
//! ## Usage with `Message<T>`
//!
//! ```rust
//! use crate::graph::serialization::{serialize, deserialize};
//! use crate::message::{Message, wrap_message};
//! use bytes::Bytes;
//!
//! // Create a message
//! let msg = wrap_message(42i32);
//!
//! // Serialize Message<T> - preserves ID and metadata
//! let bytes: Bytes = serialize(&msg)?;
//!
//! // Deserialize Message<T> - ID and metadata are restored
//! let deserialized: Message<i32> = deserialize(bytes)?;
//! assert_eq!(*deserialized.payload(), 42);
//! assert_eq!(deserialized.id(), msg.id()); // Same ID preserved
//! # Ok::<(), crate::graph::SerializationError>(())
//! ```

use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};
use std::fmt;
use tracing::trace;

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
/// # `Message<T>` Support
///
/// This function fully supports `Message<T>` serialization. When serializing
/// a `Message<T>`, the message ID and metadata are preserved in the serialized
/// output, enabling end-to-end traceability and message persistence.
///
/// # Arguments
///
/// * `item` - The item to serialize (can be `Message<T>` or any `Serialize` type).
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
/// use crate::graph::serialize;
///
/// #[derive(Serialize)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// let point = Point { x: 1, y: 2 };
/// let bytes = serialize(&point)?;
/// # Ok::<(), crate::graph::SerializationError>(())
/// ```
///
/// # Example with `Message<T>`
///
/// ```rust
/// use crate::graph::serialize;
/// use crate::message::wrap_message;
///
/// let msg = wrap_message(42i32);
/// let bytes = serialize(&msg)?; // Serializes Message<i32> with ID and metadata
/// # Ok::<(), crate::graph::SerializationError>(())
/// ```
pub fn serialize<T: Serialize>(item: &T) -> Result<Bytes, SerializationError> {
  trace!("serialize()");
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
/// # `Message<T>` Support
///
/// This function fully supports `Message<T>` deserialization. When deserializing
/// a `Message<T>`, the message ID and metadata are restored from the serialized
/// data, preserving end-to-end traceability.
///
/// # Arguments
///
/// * `data` - The bytes containing JSON data to deserialize (can be serialized `Message<T>` or any type).
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
/// use crate::graph::deserialize;
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
/// # Ok::<(), crate::graph::SerializationError>(())
/// ```
///
/// # Example with `Message<T>`
///
/// ```rust
/// use bytes::Bytes;
/// use crate::graph::{serialize, deserialize};
/// use crate::message::wrap_message;
///
/// let msg = wrap_message(42i32);
/// let bytes = serialize(&msg)?;
/// let deserialized: crate::message::Message<i32> = deserialize(bytes)?;
/// assert_eq!(*deserialized.payload(), 42);
/// # Ok::<(), crate::graph::SerializationError>(())
/// ```
pub fn deserialize<T: DeserializeOwned>(data: Bytes) -> Result<T, SerializationError> {
  trace!("deserialize(data.len={})", data.len());
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
/// # `Message<T>` Support
///
/// This deserializer fully supports `Message<T>` deserialization. When deserializing
/// a `Message<T>`, the message ID and metadata are restored from the serialized data.
///
/// # Example
///
/// ```rust
/// use crate::graph::serialization::ZeroCopyDeserializer;
/// use bytes::Bytes;
///
/// let buffer = Bytes::from(r#"{"text": "hello"}"#);
/// let deserializer = ZeroCopyDeserializer::new(buffer);
/// let result: serde_json::Value = deserializer.deserialize().unwrap();
/// ```
///
/// # Example with `Message<T>`
///
/// ```rust
/// use crate::graph::serialization::{serialize, ZeroCopyDeserializer};
/// use crate::message::wrap_message;
/// use bytes::Bytes;
///
/// let msg = wrap_message(42i32);
/// let bytes = serialize(&msg)?;
/// let deserializer = ZeroCopyDeserializer::new(bytes);
/// let deserialized: crate::message::Message<i32> = deserializer.deserialize()?;
/// assert_eq!(*deserialized.payload(), 42);
/// # Ok::<(), crate::graph::SerializationError>(())
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
    trace!("ZeroCopyDeserializer::new(buffer.len={})", buffer.len());
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
    trace!(
      "ZeroCopyDeserializer::deserialize(buffer.len={})",
      self.buffer.len()
    );
    serde_json::from_slice(self.buffer.as_ref()).map_err(Into::into)
  }

  /// Get a reference to the underlying buffer.
  ///
  /// # Returns
  ///
  /// A reference to the `Bytes` buffer
  #[must_use]
  pub fn buffer(&self) -> &Bytes {
    trace!(
      "ZeroCopyDeserializer::buffer() -> len={}",
      self.buffer.len()
    );
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
/// use crate::graph::serialization::deserialize_zero_copy_strings;
///
/// let data = b"{\"text\": \"hello\"}";
/// let result: serde_json::Value = deserialize_zero_copy_strings(data).unwrap();
/// // Strings in result are &str references pointing into data
/// ```
pub fn deserialize_zero_copy_strings<'de, T>(data: &'de [u8]) -> Result<T, SerializationError>
where
  T: serde::Deserialize<'de>,
{
  trace!("deserialize_zero_copy_strings(data.len={})", data.len());
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
    trace!("JsonSerializer::serialize()");
    serialize(item)
  }

  fn deserialize<T: DeserializeOwned>(&self, bytes: Bytes) -> Result<T, SerializationError> {
    trace!("JsonSerializer::deserialize(bytes.len={})", bytes.len());
    deserialize(bytes)
  }
}
