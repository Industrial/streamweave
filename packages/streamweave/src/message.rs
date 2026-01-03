//! Message envelope types for universal message-based processing.
//!
//! This module provides the foundation for StreamWeave's universal message model.
//! **All data in StreamWeave flows as `Message<T>`**, where `T` is the payload type.
//! This design enables end-to-end traceability, metadata preservation, and advanced
//! features like deduplication, offset tracking, and exactly-once processing.
//!
//! # Universal Message Model
//!
//! Every item in a StreamWeave pipeline is wrapped in a `Message<T>` envelope that contains:
//!
//! - **Payload**: Your actual data (`T`)
//! - **MessageId**: Unique identifier for tracking and correlation
//! - **MessageMetadata**: Timestamps, source, headers, and custom attributes
//!
//! This universal model ensures that:
//! - Messages can be tracked through complex pipelines
//! - Metadata is preserved through transformations
//! - Errors can be correlated to specific messages
//! - Zero-copy sharing is efficient in fan-out scenarios
//!
//! # Core Types
//!
//! - **[`Message<T>`]**: The universal message envelope containing payload, ID, and metadata
//! - **[`MessageId`]**: Unique identifier (UUID, Sequence, Custom, or ContentHash)
//! - **[`MessageMetadata`]**: Timestamps, source, headers, and custom attributes
//! - **[`IdGenerator`]**: Trait for generating message IDs
//! - **[`MessageStreamExt`]**: Extension trait for streams of messages
//!
//! # Quick Start
//!
//! ## Creating Messages
//!
//! ```rust
//! use streamweave::message::{Message, MessageId, wrap_message};
//!
//! // Simple message creation (auto-generates UUID)
//! let msg = wrap_message(42);
//!
//! // Message with specific ID
//! let msg = Message::new(42, MessageId::new_uuid());
//!
//! // Message with custom metadata
//! use streamweave::message::MessageMetadata;
//! let metadata = MessageMetadata::default()
//!     .source("my_source")
//!     .header("key", "value");
//! let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata);
//! ```
//!
//! ## Accessing Message Components
//!
//! ```rust
//! use streamweave::message::wrap_message;
//!
//! let msg = wrap_message(42);
//!
//! // Access payload
//! let payload = msg.payload();      // &i32
//!
//! // Access ID
//! let id = msg.id();                // &MessageId
//!
//! // Access metadata
//! let metadata = msg.metadata();    // &MessageMetadata
//! ```
//!
//! ## Transforming Messages
//!
//! ```rust
//! use streamweave::message::wrap_message;
//!
//! let msg1 = wrap_message(42);
//!
//! // Transform payload while preserving ID and metadata
//! let doubled = msg1.map(|x| x * 2);
//!
//! // Extract payload (consumes message)
//! let msg2 = wrap_message(42);
//! let value = msg2.into_payload();   // i32
//! ```
//!
//! ## Working with Message Streams
//!
//! ```rust
//! use streamweave::message::{Message, MessageId, MessageStreamExt};
//! use futures::stream;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let messages = stream::iter(vec![
//!     Message::new(1, MessageId::new_uuid()),
//!     Message::new(2, MessageId::new_uuid()),
//! ]);
//!
//! // Extract payloads
//! let payloads: Vec<i32> = messages
//!     .extract_payloads()
//!     .collect()
//!     .await;
//!
//! // Extract IDs
//! let messages2 = stream::iter(vec![
//!     Message::new(1, MessageId::new_uuid()),
//!     Message::new(2, MessageId::new_uuid()),
//! ]);
//! let ids: Vec<MessageId> = messages2
//!     .extract_ids()
//!     .collect()
//!     .await;
//! # Ok(())
//! # }
//! ```
//!
//! # Message ID Types
//!
//! Message IDs can be generated using different strategies:
//!
//! - **UUID**: Globally unique (UUIDv4), ideal for distributed systems
//! - **Sequence**: Monotonically increasing, good for ordered processing
//! - **Custom**: User-provided string identifier
//! - **ContentHash**: Hash-based identifier derived from content
//!
//! # Helper Functions
//!
//! The module provides several convenience functions:
//!
//! - `wrap_message(payload)` - Create message with auto-generated UUID
//! - `wrap_message_with_id(payload, id)` - Create message with specific ID
//! - `wrap_message_with_metadata(payload, metadata)` - Create message with metadata
//! - `wrap_messages(payloads)` - Wrap a collection of payloads
//! - `unwrap_message(message)` - Extract payload from message
//!
//! # Zero-Copy Architecture
//!
//! The message module is designed for zero-copy efficiency:
//! - `MessageMetadata` uses `Arc<str>` for string fields to enable sharing
//! - Messages can be shared efficiently using `Arc<Message<T>>` or `SharedMessage<T>`
//! - Metadata headers use `Arc<str>` for both keys and values

use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Re-export for convenience methods
use futures::StreamExt;

// Serde support for serialization
use serde::{Deserialize, Serialize};

/// A unique identifier for messages.
///
/// `MessageId` provides several strategies for generating unique identifiers,
/// each suited for different use cases:
///
/// - **UUID**: Globally unique (UUIDv4), ideal for distributed systems where
///   uniqueness across multiple nodes is required
/// - **Sequence**: Monotonically increasing numbers, good for ordered processing
///   and maintaining sequence in single-process scenarios
/// - **Custom**: User-provided string identifier, useful when integrating with
///   external systems that provide their own IDs
/// - **ContentHash**: Hash-based identifier derived from message content,
///   useful for deduplication based on content
///
/// # Examples
///
/// ```rust
/// use streamweave::message::MessageId;
///
/// // UUID-based ID (most common)
/// let id = MessageId::new_uuid();
///
/// // Sequence-based ID
/// let id = MessageId::new_sequence(42);
///
/// // Custom string ID
/// let id = MessageId::new_custom("my-custom-id");
///
/// // Content-hash ID
/// let id = MessageId::from_content(b"some content");
/// ```
///
/// # ID Type Checking
///
/// ```rust
/// use streamweave::message::MessageId;
///
/// let id = MessageId::new_uuid();
/// assert!(id.is_uuid());
/// assert!(!id.is_sequence());
/// ```
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MessageId {
  /// A UUID-based identifier (128-bit).
  Uuid(u128),

  /// A sequence-based identifier (64-bit).
  Sequence(u64),

  /// A custom string identifier.
  Custom(String),

  /// A hash-based identifier derived from content.
  ContentHash(u64),
}

impl MessageId {
  /// Create a new UUID-based message ID using UUIDv4.
  #[must_use]
  pub fn new_uuid() -> Self {
    // Use a simple random UUID implementation
    let high = rand_u64();
    let low = rand_u64();
    // Set version 4 and variant bits
    let uuid = ((high & 0xFFFFFFFFFFFF0FFF) | 0x0000000000004000) as u128
      | (((low & 0x3FFFFFFFFFFFFFFF) | 0x8000000000000000) as u128) << 64;
    MessageId::Uuid(uuid)
  }

  /// Create a new sequence-based message ID.
  #[must_use]
  pub const fn new_sequence(seq: u64) -> Self {
    MessageId::Sequence(seq)
  }

  /// Create a custom message ID from a string.
  #[must_use]
  pub fn new_custom(id: impl Into<String>) -> Self {
    MessageId::Custom(id.into())
  }

  /// Create a content-hash message ID from the given bytes.
  #[must_use]
  pub fn from_content(content: &[u8]) -> Self {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    MessageId::ContentHash(hasher.finish())
  }

  /// Returns true if this is a UUID-based ID.
  #[must_use]
  pub const fn is_uuid(&self) -> bool {
    matches!(self, MessageId::Uuid(_))
  }

  /// Returns true if this is a sequence-based ID.
  #[must_use]
  pub const fn is_sequence(&self) -> bool {
    matches!(self, MessageId::Sequence(_))
  }

  /// Returns true if this is a custom ID.
  #[must_use]
  pub const fn is_custom(&self) -> bool {
    matches!(self, MessageId::Custom(_))
  }

  /// Returns true if this is a content-hash ID.
  #[must_use]
  pub const fn is_content_hash(&self) -> bool {
    matches!(self, MessageId::ContentHash(_))
  }
}

impl Display for MessageId {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      MessageId::Uuid(uuid) => {
        // Format as standard UUID string
        write!(
          f,
          "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
          (uuid >> 96) as u32,
          (uuid >> 80) as u16,
          (uuid >> 64) as u16,
          (uuid >> 48) as u16,
          (uuid & 0xFFFFFFFFFFFF) as u64
        )
      }
      MessageId::Sequence(seq) => write!(f, "seq:{}", seq),
      MessageId::Custom(id) => write!(f, "custom:{}", id),
      MessageId::ContentHash(hash) => write!(f, "hash:{:016x}", hash),
    }
  }
}

impl Hash for MessageId {
  fn hash<H: Hasher>(&self, state: &mut H) {
    std::mem::discriminant(self).hash(state);
    match self {
      MessageId::Uuid(uuid) => uuid.hash(state),
      MessageId::Sequence(seq) => seq.hash(state),
      MessageId::Custom(id) => id.hash(state),
      MessageId::ContentHash(hash) => hash.hash(state),
    }
  }
}

impl Default for MessageId {
  fn default() -> Self {
    MessageId::new_uuid()
  }
}

/// Metadata associated with a message.
///
/// `MessageMetadata` provides additional context about a message, including
/// timestamps, source information, routing keys, and custom headers. This
/// metadata is preserved through transformations, enabling end-to-end traceability.
///
/// ## Zero-Copy Design
///
/// Uses `Arc<str>` for string fields to enable zero-copy string sharing
/// in fan-out scenarios and reduce memory usage for repeated strings.
///
/// ## Metadata Fields
///
/// - **timestamp**: When the message was created (Duration since UNIX_EPOCH)
/// - **source**: The source of the message (e.g., topic, file, database table)
/// - **partition**: Partition or shard information
/// - **offset**: Offset within the partition/source
/// - **key**: User-defined key for routing/grouping
/// - **headers**: Additional key-value pairs for custom attributes
///
/// # Examples
///
/// ## Basic Metadata
///
/// ```rust
/// use streamweave::message::MessageMetadata;
///
/// // Create metadata with source
/// let metadata = MessageMetadata::default()
///     .source("my_topic")
///     .header("content-type", "application/json");
/// ```
///
/// ## Metadata with Timestamp
///
/// ```rust
/// use streamweave::message::MessageMetadata;
///
/// // Create metadata with current timestamp
/// let metadata = MessageMetadata::with_timestamp_now()
///     .source("my_source")
///     .key("user-123");
/// ```
///
/// ## Accessing Metadata
///
/// ```rust
/// use streamweave::message::MessageMetadata;
///
/// let metadata = MessageMetadata::default()
///     .source("my_source")
///     .header("key", "value");
///
/// // Access source
/// let source = metadata.get_source();  // Option<&str>
///
/// // Access headers
/// let header_value = metadata.get_header("key");  // Option<&str>
/// ```
///
/// ## Builder Pattern
///
/// All metadata methods return `Self` for method chaining:
///
/// ```rust
/// use streamweave::message::MessageMetadata;
///
/// let metadata = MessageMetadata::default()
///     .source("kafka-topic")
///     .partition(0)
///     .offset(12345)
///     .key("user-123")
///     .header("trace-id", "abc-123")
///     .header("span-id", "def-456");
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MessageMetadata {
  /// When the message was created (as Duration since UNIX_EPOCH).
  pub timestamp: Option<Duration>,

  /// The source of the message (e.g., topic, file, etc.).
  /// Uses `Arc<str>` for zero-copy string sharing.
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "serialize_arc_str_option",
    deserialize_with = "deserialize_arc_str_option"
  )]
  pub source: Option<Arc<str>>,

  /// Partition or shard information.
  pub partition: Option<u32>,

  /// Offset within the partition/source.
  pub offset: Option<u64>,

  /// User-defined key for routing/grouping.
  /// Uses `Arc<str>` for zero-copy string sharing.
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "serialize_arc_str_option",
    deserialize_with = "deserialize_arc_str_option"
  )]
  pub key: Option<Arc<str>>,

  /// Additional headers/attributes.
  /// Uses `Arc<str>` for both keys and values to enable zero-copy sharing.
  #[serde(
    serialize_with = "serialize_arc_str_vec",
    deserialize_with = "deserialize_arc_str_vec"
  )]
  pub headers: Vec<(Arc<str>, Arc<str>)>,
}

// Custom serialization helpers for Arc<str>
fn serialize_arc_str_option<S>(opt: &Option<Arc<str>>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  match opt {
    Some(arc) => serializer.serialize_some(arc.as_ref()),
    None => serializer.serialize_none(),
  }
}

fn deserialize_arc_str_option<'de, D>(deserializer: D) -> Result<Option<Arc<str>>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let opt: Option<String> = serde::Deserialize::deserialize(deserializer)?;
  Ok(opt.map(Arc::from))
}

fn serialize_arc_str_vec<S>(vec: &[(Arc<str>, Arc<str>)], serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  use serde::ser::SerializeSeq;
  let mut seq = serializer.serialize_seq(Some(vec.len()))?;
  for (k, v) in vec {
    seq.serialize_element(&(k.as_ref(), v.as_ref()))?;
  }
  seq.end()
}

type ArcStrPair = (Arc<str>, Arc<str>);

fn deserialize_arc_str_vec<'de, D>(deserializer: D) -> Result<Vec<ArcStrPair>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let vec: Vec<(String, String)> = serde::Deserialize::deserialize(deserializer)?;
  Ok(
    vec
      .into_iter()
      .map(|(k, v)| (Arc::from(k), Arc::from(v)))
      .collect(),
  )
}

impl MessageMetadata {
  /// Create new empty metadata.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Create metadata with the current timestamp.
  #[must_use]
  pub fn with_timestamp_now() -> Self {
    Self {
      timestamp: SystemTime::now().duration_since(UNIX_EPOCH).ok(),
      ..Default::default()
    }
  }

  /// Set the timestamp.
  #[must_use]
  pub fn timestamp(mut self, ts: Duration) -> Self {
    self.timestamp = Some(ts);
    self
  }

  /// Set the source.
  ///
  /// Accepts any type that can be converted to `Arc<str>`, including
  /// `String`, `&str`, and `Arc<str>`.
  #[must_use]
  pub fn source(mut self, source: impl Into<Arc<str>>) -> Self {
    self.source = Some(source.into());
    self
  }

  /// Set the partition.
  #[must_use]
  pub fn partition(mut self, partition: u32) -> Self {
    self.partition = Some(partition);
    self
  }

  /// Set the offset.
  #[must_use]
  pub fn offset(mut self, offset: u64) -> Self {
    self.offset = Some(offset);
    self
  }

  /// Set the key.
  ///
  /// Accepts any type that can be converted to `Arc<str>`, including
  /// `String`, `&str`, and `Arc<str>`.
  #[must_use]
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Add a header.
  ///
  /// Accepts any types that can be converted to `Arc<str>`, including
  /// `String`, `&str`, and `Arc<str>`.
  #[must_use]
  pub fn header(mut self, name: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
    self.headers.push((name.into(), value.into()));
    self
  }

  /// Get a header by name.
  ///
  /// Returns a `&str` reference to the header value if found.
  #[must_use]
  pub fn get_header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(k, _)| k.as_ref() == name)
      .map(|(_, v)| v.as_ref())
  }

  /// Get the source as a string reference.
  ///
  /// Returns `None` if no source is set, or `Some(&str)` with the source value.
  #[must_use]
  pub fn get_source(&self) -> Option<&str> {
    self.source.as_deref()
  }

  /// Get the key as a string reference.
  ///
  /// Returns `None` if no key is set, or `Some(&str)` with the key value.
  #[must_use]
  pub fn get_key(&self) -> Option<&str> {
    self.key.as_deref()
  }

  /// Create metadata with shared source (for zero-copy scenarios).
  ///
  /// This method accepts an `Arc<str>` for the source, enabling zero-copy
  /// sharing of source strings across multiple messages.
  ///
  /// This is now equivalent to `source()` since MessageMetadata uses `Arc<str>` directly.
  #[must_use]
  pub fn with_shared_source(mut self, source: Arc<str>) -> Self {
    self.source = Some(source);
    self
  }

  /// Create metadata with shared key (for zero-copy scenarios).
  ///
  /// This method accepts an `Arc<str>` for the key, enabling zero-copy
  /// sharing of key strings across multiple messages.
  ///
  /// This is now equivalent to `key()` since MessageMetadata uses `Arc<str>` directly.
  #[must_use]
  pub fn with_shared_key(mut self, key: Arc<str>) -> Self {
    self.key = Some(key);
    self
  }

  /// Add a header with shared strings (for zero-copy scenarios).
  ///
  /// This method accepts `Arc<str>` for header name and value, enabling
  /// zero-copy sharing of header strings across multiple messages.
  ///
  /// This is now equivalent to `header()` since MessageMetadata uses `Arc<str>` directly.
  #[must_use]
  pub fn with_shared_header(mut self, name: Arc<str>, value: Arc<str>) -> Self {
    self.headers.push((name, value));
    self
  }

  /// Create metadata with source from a borrowed string.
  ///
  /// This is a convenience method that accepts `&str` and converts it to `Arc<str>`.
  #[must_use]
  pub fn with_source_borrowed(mut self, source: &str) -> Self {
    self.source = Some(Arc::from(source));
    self
  }

  /// Create metadata with interned source string.
  ///
  /// This method accepts a `&str` and a string interner, interns the string,
  /// and stores the interned `Arc<str>` as the source. This enables automatic
  /// string deduplication for repeated source values.
  ///
  /// # Arguments
  ///
  /// * `source` - The source string to intern
  /// * `interner` - The string interner to use (must implement `StringInternerTrait`)
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  ///
  /// # Note
  ///
  /// This method accepts any type that implements `StringInternerTrait`, which
  /// is provided by `streamweave::graph::StringInterner`. For use with the graph
  /// module, pass a `&StringInterner` from `streamweave::graph`.
  #[must_use]
  pub fn with_source_interned<I: StringInternerTrait>(
    mut self,
    source: &str,
    interner: &I,
  ) -> Self {
    self.source = Some(interner.get_or_intern(source));
    self
  }

  /// Create metadata with interned key string.
  ///
  /// This method accepts a `&str` and a string interner, interns the string,
  /// and stores the interned `Arc<str>` as the key. This enables automatic
  /// string deduplication for repeated key values.
  ///
  /// # Arguments
  ///
  /// * `key` - The key string to intern
  /// * `interner` - The string interner to use (must implement `StringInternerTrait`)
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  #[must_use]
  pub fn with_key_interned<I: StringInternerTrait>(mut self, key: &str, interner: &I) -> Self {
    self.key = Some(interner.get_or_intern(key));
    self
  }

  /// Add a header with interned strings.
  ///
  /// This method accepts header name and value as `&str`, interns both strings,
  /// and adds the interned `Arc<str>` pair to headers. This enables automatic
  /// string deduplication for repeated header keys and values.
  ///
  /// # Arguments
  ///
  /// * `name` - The header name to intern
  /// * `value` - The header value to intern
  /// * `interner` - The string interner to use (must implement `StringInternerTrait`)
  ///
  /// # Returns
  ///
  /// `Self` for method chaining
  #[must_use]
  pub fn add_header_interned<I: StringInternerTrait>(
    mut self,
    name: &str,
    value: &str,
    interner: &I,
  ) -> Self {
    self
      .headers
      .push((interner.get_or_intern(name), interner.get_or_intern(value)));
    self
  }
}

/// A message envelope that wraps a payload with an ID and metadata.
///
/// `Message<T>` is the universal type for all data flowing through StreamWeave pipelines.
/// Every item in a pipeline is wrapped in a `Message<T>` to enable:
/// - **End-to-end traceability**: Track messages through complex pipelines using `MessageId`
/// - **Metadata preservation**: Pass context (source, headers, timestamps) through transformations
/// - **Error correlation**: Link errors to specific messages for debugging
/// - **Zero-copy sharing**: Efficient message sharing in fan-out scenarios
///
/// # Type Parameters
///
/// - `T`: The payload type (must implement `Debug + Clone + Send + Sync + 'static`)
///
/// # Message Components
///
/// Every `Message<T>` contains:
/// - **Payload**: Your actual data (`T`)
/// - **MessageId**: Unique identifier (UUID, Sequence, Custom, or ContentHash)
/// - **MessageMetadata**: Timestamps, source, headers, and custom attributes
///
/// # Creating Messages
///
/// ```rust
/// use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};
///
/// // Simple creation with auto-generated UUID
/// let msg = wrap_message(42);
///
/// // Create with specific ID
/// let msg = Message::new(42, MessageId::new_uuid());
///
/// // Create with custom metadata
/// let metadata = MessageMetadata::default()
///     .source("my_source")
///     .header("key", "value");
/// let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata);
/// ```
///
/// # Accessing Message Components
///
/// ```rust
/// use streamweave::message::wrap_message;
///
/// let msg = wrap_message(42);
///
/// // Access payload (borrowed)
/// let payload = msg.payload();      // &i32
///
/// // Access ID
/// let id = msg.id();                // &MessageId
///
/// // Access metadata
/// let metadata = msg.metadata();    // &MessageMetadata
/// ```
///
/// # Transforming Messages
///
/// ```rust
/// use streamweave::message::wrap_message;
///
/// let msg1 = wrap_message(42);
///
/// // Transform payload while preserving ID and metadata
/// let doubled = msg1.map(|x| x * 2);
///
/// // Extract payload (consumes message)
/// let msg2 = wrap_message(42);
/// let value = msg2.into_payload();   // i32
/// ```
///
/// # Message Preservation Through Transformations
///
/// When transforming messages, always preserve the ID and metadata:
///
/// ```rust,no_run
/// use streamweave::message::Message;
/// use futures::StreamExt;
///
/// // In a transformer:
/// // stream.map(|msg| {
/// //     let payload = msg.payload().clone();
/// //     let id = msg.id().clone();
/// //     let metadata = msg.metadata().clone();
/// //     Message::with_metadata(payload * 2, id, metadata)
/// // })
/// ```
///
/// # Zero-Copy Sharing
///
/// Messages can be shared efficiently using `Arc`:
///
/// ```rust
/// use streamweave::message::{wrap_message, Message};
/// use std::sync::Arc;
///
/// let msg = wrap_message(42);
/// let shared: Arc<Message<i32>> = Arc::new(msg);
/// // Multiple consumers can share the same message efficiently
/// ```
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(
  serialize = "T: serde::Serialize",
  deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct Message<T> {
  id: MessageId,
  payload: T,
  metadata: MessageMetadata,
}

impl<T> Message<T> {
  /// Create a new message with the given payload and ID.
  #[must_use]
  pub fn new(payload: T, id: MessageId) -> Self {
    Self {
      id,
      payload,
      metadata: MessageMetadata::with_timestamp_now(),
    }
  }

  /// Create a new message with payload, ID, and metadata.
  #[must_use]
  pub fn with_metadata(payload: T, id: MessageId, metadata: MessageMetadata) -> Self {
    Self {
      id,
      payload,
      metadata,
    }
  }

  /// Get the message ID.
  #[must_use]
  pub fn id(&self) -> &MessageId {
    &self.id
  }

  /// Get the payload.
  #[must_use]
  pub fn payload(&self) -> &T {
    &self.payload
  }

  /// Get a mutable reference to the payload.
  pub fn payload_mut(&mut self) -> &mut T {
    &mut self.payload
  }

  /// Get the metadata.
  #[must_use]
  pub fn metadata(&self) -> &MessageMetadata {
    &self.metadata
  }

  /// Get a mutable reference to the metadata.
  pub fn metadata_mut(&mut self) -> &mut MessageMetadata {
    &mut self.metadata
  }

  /// Consume the message and return its components.
  #[must_use]
  pub fn into_parts(self) -> (MessageId, T, MessageMetadata) {
    (self.id, self.payload, self.metadata)
  }

  /// Consume the message and return just the payload.
  #[must_use]
  pub fn into_payload(self) -> T {
    self.payload
  }

  /// Map the payload to a new type.
  #[must_use]
  pub fn map<U, F>(self, f: F) -> Message<U>
  where
    F: FnOnce(T) -> U,
  {
    Message {
      id: self.id,
      payload: f(self.payload),
      metadata: self.metadata,
    }
  }

  /// Map the payload to a new type, with access to the message ID.
  #[must_use]
  pub fn map_with_id<U, F>(self, f: F) -> Message<U>
  where
    F: FnOnce(&MessageId, T) -> U,
  {
    Message {
      payload: f(&self.id, self.payload),
      id: self.id,
      metadata: self.metadata,
    }
  }

  /// Replace the payload with a new value.
  #[must_use]
  pub fn with_payload<U>(self, payload: U) -> Message<U> {
    Message {
      id: self.id,
      payload,
      metadata: self.metadata,
    }
  }
}

impl<T: Default> Default for Message<T> {
  fn default() -> Self {
    Self::new(T::default(), MessageId::new_uuid())
  }
}

impl<T: PartialEq> PartialEq for Message<T> {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id && self.payload == other.payload
  }
}

impl<T: Eq> Eq for Message<T> {}

impl<T: Hash> Hash for Message<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.id.hash(state);
    self.payload.hash(state);
  }
}

/// Trait for types that generate message IDs.
/// Trait for generating unique message IDs.
///
/// Implementations of this trait provide different strategies for generating
/// message identifiers. The trait is used by adapters and helper functions
/// to automatically generate IDs for messages.
///
/// # Implementations
///
/// - [`UuidGenerator`]: Generates UUIDv4-style identifiers
/// - [`SequenceGenerator`]: Generates monotonically increasing sequence numbers
///
/// # Example
///
/// ```rust
/// use streamweave::message::{IdGenerator, UuidGenerator, SequenceGenerator};
///
/// // UUID generator
/// let uuid_gen = UuidGenerator::new();
/// let id1 = uuid_gen.next_id();
/// let id2 = uuid_gen.next_id();
///
/// // Sequence generator
/// let seq_gen = SequenceGenerator::new();
/// let id1 = seq_gen.next_id();  // 0
/// let id2 = seq_gen.next_id();  // 1
/// ```
///
/// # Custom Implementations
///
/// You can implement `IdGenerator` for custom ID generation strategies:
///
/// ```rust,no_run
/// use streamweave::message::{IdGenerator, MessageId};
///
/// struct CustomGenerator {
///     counter: std::sync::atomic::AtomicU64,
/// }
///
/// impl IdGenerator for CustomGenerator {
///     fn next_id(&self) -> MessageId {
///         let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
///         MessageId::new_custom(format!("custom-{}", count))
///     }
/// }
/// ```
pub trait IdGenerator: Send + Sync {
  /// Generate the next message ID.
  ///
  /// This method should return a unique identifier. The exact uniqueness
  /// guarantees depend on the implementation:
  ///
  /// - `UuidGenerator`: Globally unique (UUIDv4)
  /// - `SequenceGenerator`: Unique within the generator instance
  /// - Custom implementations: Depends on implementation
  fn next_id(&self) -> MessageId;
}

/// UUID-based ID generator.
///
/// Generates unique UUIDv4-style identifiers.
#[derive(Debug, Default)]
pub struct UuidGenerator;

impl UuidGenerator {
  /// Create a new UUID generator.
  #[must_use]
  pub fn new() -> Self {
    Self
  }
}

impl IdGenerator for UuidGenerator {
  fn next_id(&self) -> MessageId {
    MessageId::new_uuid()
  }
}

/// Sequence-based ID generator.
///
/// Generates monotonically increasing sequence numbers.
/// Thread-safe using atomic operations.
#[derive(Debug)]
pub struct SequenceGenerator {
  counter: AtomicU64,
}

impl SequenceGenerator {
  /// Create a new sequence generator starting at 0.
  #[must_use]
  pub fn new() -> Self {
    Self {
      counter: AtomicU64::new(0),
    }
  }

  /// Create a new sequence generator starting at the given value.
  #[must_use]
  pub fn starting_at(start: u64) -> Self {
    Self {
      counter: AtomicU64::new(start),
    }
  }

  /// Get the current sequence number without incrementing.
  #[must_use]
  pub fn current(&self) -> u64 {
    self.counter.load(Ordering::Relaxed)
  }

  /// Reset the sequence to 0.
  pub fn reset(&self) {
    self.counter.store(0, Ordering::Relaxed);
  }

  /// Reset the sequence to a specific value.
  pub fn reset_to(&self, value: u64) {
    self.counter.store(value, Ordering::Relaxed);
  }
}

impl Default for SequenceGenerator {
  fn default() -> Self {
    Self::new()
  }
}

impl IdGenerator for SequenceGenerator {
  fn next_id(&self) -> MessageId {
    MessageId::Sequence(self.counter.fetch_add(1, Ordering::Relaxed))
  }
}

/// Content-hash ID generator.
///
/// Generates IDs based on the content of the message.
/// This is useful for idempotency based on message content.
#[derive(Debug, Default)]
pub struct ContentHashGenerator;

impl ContentHashGenerator {
  /// Create a new content hash generator.
  #[must_use]
  pub fn new() -> Self {
    Self
  }

  /// Generate an ID from the given content.
  #[must_use]
  pub fn hash_content(&self, content: &[u8]) -> MessageId {
    MessageId::from_content(content)
  }
}

/// A shared ID generator that can be cloned across threads.
pub type SharedIdGenerator = Arc<dyn IdGenerator>;

/// Create a shared UUID generator.
#[must_use]
pub fn uuid_generator() -> SharedIdGenerator {
  Arc::new(UuidGenerator::new())
}

/// Create a shared sequence generator.
#[must_use]
pub fn sequence_generator() -> SharedIdGenerator {
  Arc::new(SequenceGenerator::new())
}

/// Create a shared sequence generator starting at the given value.
#[must_use]
pub fn sequence_generator_from(start: u64) -> SharedIdGenerator {
  Arc::new(SequenceGenerator::starting_at(start))
}

// Simple random number generator for UUIDs
// This uses a basic xorshift algorithm seeded from system time
fn rand_u64() -> u64 {
  use std::cell::Cell;
  use std::hash::{Hash, Hasher};

  thread_local! {
    static STATE: Cell<u64> = {
      // Seed from time and thread ID (hashed for uniqueness)
      let time_seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x12345678DEADBEEF);

      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      std::thread::current().id().hash(&mut hasher);
      let thread_seed = hasher.finish();

      Cell::new(time_seed ^ thread_seed)
    };
  }

  STATE.with(|state| {
    let mut x = state.get();
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    state.set(x);
    x
  })
}

/// A shared message wrapper for zero-copy message sharing.
///
/// This type wraps `Arc<Message<T>>` to enable zero-copy sharing of messages
/// in fan-out scenarios where one message needs to be sent to multiple consumers.
///
/// # Example
///
/// ```rust
/// use streamweave::message::{Message, MessageId, SharedMessage};
///
/// let msg = Message::new(42, MessageId::new_uuid());
/// let shared = SharedMessage::from(msg);
/// let cloned = shared.clone(); // Zero-cost clone of Arc
/// ```
#[derive(Clone, Debug)]
pub struct SharedMessage<T> {
  inner: Arc<Message<T>>,
}

impl<T> SharedMessage<T> {
  /// Create a SharedMessage from an owned Message.
  #[must_use]
  pub fn from(message: Message<T>) -> Self {
    Self {
      inner: Arc::new(message),
    }
  }

  /// Create a SharedMessage from an `Arc<Message<T>>`.
  #[must_use]
  pub fn from_arc(arc: Arc<Message<T>>) -> Self {
    Self { inner: arc }
  }

  /// Get the inner Arc.
  #[must_use]
  pub fn into_arc(self) -> Arc<Message<T>> {
    self.inner
  }

  /// Try to unwrap the Arc, returning the owned Message if this is the only reference.
  ///
  /// Returns `Ok(Message<T>)` if this is the only reference, `Err(SharedMessage<T>)` otherwise.
  pub fn try_unwrap(self) -> Result<Message<T>, Self> {
    Arc::try_unwrap(self.inner).map_err(|arc| Self { inner: arc })
  }

  /// Get the message ID.
  #[must_use]
  pub fn id(&self) -> &MessageId {
    self.inner.id()
  }

  /// Get the payload.
  #[must_use]
  pub fn payload(&self) -> &T {
    self.inner.payload()
  }

  /// Get the metadata.
  #[must_use]
  pub fn metadata(&self) -> &MessageMetadata {
    self.inner.metadata()
  }
}

impl<T> std::ops::Deref for SharedMessage<T> {
  type Target = Message<T>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> std::convert::AsRef<Message<T>> for SharedMessage<T> {
  fn as_ref(&self) -> &Message<T> {
    &self.inner
  }
}

impl<T: PartialEq> PartialEq for SharedMessage<T> {
  fn eq(&self, other: &Self) -> bool {
    self.inner.as_ref() == other.inner.as_ref()
  }
}

impl<T: Eq> Eq for SharedMessage<T> {}

impl<T: Hash> Hash for SharedMessage<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inner.hash(state);
  }
}

impl<T> From<Message<T>> for SharedMessage<T> {
  fn from(message: Message<T>) -> Self {
    Self::from(message)
  }
}

impl<T> From<Arc<Message<T>>> for SharedMessage<T> {
  fn from(arc: Arc<Message<T>>) -> Self {
    Self::from_arc(arc)
  }
}

impl<T> From<SharedMessage<T>> for Arc<Message<T>> {
  fn from(shared: SharedMessage<T>) -> Self {
    shared.into_arc()
  }
}

// Note: ZeroCopyShare trait implementation for Message<T> is provided by the
// blanket implementation in streamweave::graph for all types that are
// Clone + Send + Sync + 'static. Message<T> automatically implements
// ZeroCopyShare when used with the graph package, enabling zero-copy
// sharing via Arc<Message<T>> in fan-out scenarios.

// Note: Arc::from() already provides conversion from String and &str to Arc<str>
// No need to implement From trait due to orphan rule restrictions

/// Trait for string interning functionality.
///
/// This trait abstracts over string interner implementations, allowing
/// `MessageMetadata` to work with any interner type. The `streamweave::graph`
/// package provides a concrete `StringInterner` implementation that implements
/// this trait.
pub trait StringInternerTrait {
  /// Get an interned string, or intern a new one if it doesn't exist.
  ///
  /// # Arguments
  ///
  /// * `s` - The string to intern or retrieve
  ///
  /// # Returns
  ///
  /// An `Arc<str>` containing the interned string
  fn get_or_intern(&self, s: &str) -> Arc<str>;
}

/// Extension trait for streams of messages that provides convenience methods
/// for accessing message components.
///
/// This trait provides ergonomic methods for working with streams of `Message<T>`,
/// allowing easy access to payloads, IDs, and metadata.
///
/// # Example
///
/// ```rust
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use streamweave::message::{Message, MessageId, MessageStreamExt};
/// use futures::stream;
/// use futures::StreamExt;
///
/// let messages1 = stream::iter(vec![
///     Message::new(1, MessageId::new_uuid()),
///     Message::new(2, MessageId::new_uuid()),
/// ]);
///
/// // Extract payloads
/// let payloads: Vec<i32> = messages1
///     .extract_payloads()
///     .collect()
///     .await;
///
/// let messages2 = stream::iter(vec![
///     Message::new(1, MessageId::new_uuid()),
///     Message::new(2, MessageId::new_uuid()),
/// ]);
///
/// // Extract IDs
/// let ids: Vec<MessageId> = messages2
///     .extract_ids()
///     .collect()
///     .await;
/// # Ok(())
/// # }
/// ```
pub trait MessageStreamExt<T>: futures::Stream<Item = Message<T>> + Sized + Send {
  /// Extract payloads from a stream of messages.
  ///
  /// # Returns
  ///
  /// A stream that yields the payloads from each message.
  fn extract_payloads(self) -> impl futures::Stream<Item = T> + Send
  where
    T: Send,
  {
    self.map(|msg| msg.into_payload())
  }

  /// Extract message IDs from a stream of messages.
  ///
  /// # Returns
  ///
  /// A stream that yields the ID from each message.
  fn extract_ids(self) -> impl futures::Stream<Item = MessageId> + Send {
    self.map(|msg| msg.id().clone())
  }

  /// Extract metadata from a stream of messages.
  ///
  /// # Returns
  ///
  /// A stream that yields the metadata from each message.
  fn extract_metadata(self) -> impl futures::Stream<Item = MessageMetadata> + Send {
    self.map(|msg| msg.metadata().clone())
  }

  /// Map over message payloads while preserving message structure.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that transforms the payload
  ///
  /// # Returns
  ///
  /// A stream of messages with transformed payloads, preserving IDs and metadata.
  fn map_payload<U, F>(self, f: F) -> impl futures::Stream<Item = Message<U>> + Send
  where
    F: Fn(T) -> U + Send + Sync + 'static,
    U: std::fmt::Debug + Clone + Send + Sync + 'static,
    T: Send,
  {
    self.map(move |msg| msg.map(&f))
  }
}

// Blanket implementation for all streams of Message<T>
impl<T, S> MessageStreamExt<T> for S
where
  S: futures::Stream<Item = Message<T>> + Send + Sized,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
}

// ============================================================================
// Helper Functions for Message Creation
// ============================================================================

/// Wrap a raw value into a `Message<T>` with a generated UUID.
///
/// This is a convenience function for creating messages from raw values.
/// It automatically generates a UUID for the message ID and sets the current timestamp.
///
/// # Arguments
///
/// * `payload` - The payload value to wrap
///
/// # Returns
///
/// A new `Message<T>` with the given payload, a UUID ID, and current timestamp.
///
/// # Example
///
/// ```rust
/// use streamweave::message::wrap_message;
///
/// let msg = wrap_message(42);
/// assert_eq!(*msg.payload(), 42);
/// ```
#[must_use]
pub fn wrap_message<T>(payload: T) -> Message<T> {
  Message::new(payload, MessageId::new_uuid())
}

/// Wrap a raw value into a `Message<T>` with a custom message ID.
///
/// # Arguments
///
/// * `payload` - The payload value to wrap
/// * `id` - The message ID to use
///
/// # Returns
///
/// A new `Message<T>` with the given payload and ID.
///
/// # Example
///
/// ```rust
/// use streamweave::message::{wrap_message_with_id, MessageId};
///
/// let id = MessageId::new_sequence(1);
/// let msg = wrap_message_with_id(42, id);
/// assert_eq!(*msg.payload(), 42);
/// ```
#[must_use]
pub fn wrap_message_with_id<T>(payload: T, id: MessageId) -> Message<T> {
  Message::new(payload, id)
}

/// Wrap a raw value into a `Message<T>` with metadata.
///
/// # Arguments
///
/// * `payload` - The payload value to wrap
/// * `id` - The message ID to use
/// * `metadata` - The message metadata
///
/// # Returns
///
/// A new `Message<T>` with the given payload, ID, and metadata.
///
/// # Example
///
/// ```rust
/// use streamweave::message::{wrap_message_with_metadata, MessageId, MessageMetadata};
///
/// let id = MessageId::new_uuid();
/// let metadata = MessageMetadata::with_timestamp_now().source("my-source");
/// let msg = wrap_message_with_metadata(42, id, metadata);
/// ```
#[must_use]
pub fn wrap_message_with_metadata<T>(
  payload: T,
  id: MessageId,
  metadata: MessageMetadata,
) -> Message<T> {
  Message::with_metadata(payload, id, metadata)
}

/// Extract the payload from a `Message<T>`.
///
/// This is a convenience function that calls `Message::into_payload()`.
///
/// # Arguments
///
/// * `message` - The message to extract the payload from
///
/// # Returns
///
/// The payload value.
///
/// # Example
///
/// ```rust
/// use streamweave::message::{wrap_message, unwrap_message};
///
/// let msg = wrap_message(42);
/// let payload = unwrap_message(msg);
/// assert_eq!(payload, 42);
/// ```
#[must_use]
pub fn unwrap_message<T>(message: Message<T>) -> T {
  message.into_payload()
}

/// Create a message using a custom ID generator.
///
/// This function uses the provided ID generator to create a new message ID
/// for the payload.
///
/// # Arguments
///
/// * `payload` - The payload value to wrap
/// * `id_generator` - The ID generator to use
///
/// # Returns
///
/// A new `Message<T>` with the given payload and a generated ID.
///
/// # Example
///
/// ```rust
/// use streamweave::message::{wrap_message_with_generator, SequenceGenerator};
///
/// let generator = SequenceGenerator::new();
/// let msg = wrap_message_with_generator(42, &generator);
/// ```
#[must_use]
pub fn wrap_message_with_generator<T>(payload: T, id_generator: &dyn IdGenerator) -> Message<T> {
  Message::new(payload, id_generator.next_id())
}

/// Create multiple messages from a collection of payloads using a shared ID generator.
///
/// This function is useful when you want to create multiple messages with
/// sequential or related IDs.
///
/// # Arguments
///
/// * `payloads` - An iterator of payload values
/// * `id_generator` - The ID generator to use for all messages
///
/// # Returns
///
/// A vector of `Message<T>` with generated IDs.
///
/// # Example
///
/// ```rust
/// use streamweave::message::{wrap_messages, SequenceGenerator};
///
/// let generator = SequenceGenerator::new();
/// let messages = wrap_messages(vec![1, 2, 3], &generator);
/// assert_eq!(messages.len(), 3);
/// ```
#[must_use]
pub fn wrap_messages<T, I>(payloads: I, id_generator: &dyn IdGenerator) -> Vec<Message<T>>
where
  I: IntoIterator<Item = T>,
{
  payloads
    .into_iter()
    .map(|payload| wrap_message_with_generator(payload, id_generator))
    .collect()
}

/// Create a message with a shared ID generator (Arc-wrapped).
///
/// This is useful when you need to share an ID generator across multiple
/// message creation calls.
///
/// # Arguments
///
/// * `payload` - The payload value to wrap
/// * `id_generator` - A shared ID generator
///
/// # Returns
///
/// A new `Message<T>` with the given payload and a generated ID.
///
/// # Example
///
/// ```rust,ignore
/// use streamweave::message::{wrap_message_with_shared_generator, SequenceGenerator};
/// use std::sync::Arc;
///
/// let generator = Arc::new(SequenceGenerator::new());
/// let msg = wrap_message_with_shared_generator(42, &*generator);
/// ```
#[must_use]
pub fn wrap_message_with_shared_generator<T>(
  payload: T,
  id_generator: &SharedIdGenerator,
) -> Message<T> {
  Message::new(payload, id_generator.next_id())
}
