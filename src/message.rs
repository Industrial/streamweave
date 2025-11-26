//! Message envelope types for exactly-once processing.
//!
//! This module provides types for wrapping stream items with unique identifiers
//! and metadata, enabling features like deduplication, offset tracking, and
//! exactly-once processing guarantees.
//!
//! # Overview
//!
//! The core types are:
//!
//! - [`MessageId`]: A unique identifier for messages
//! - [`Message<T>`]: A wrapper that adds an ID and metadata to a payload
//! - [`MessageMetadata`]: Additional information about a message
//!
//! # Example
//!
//! ```rust
//! use streamweave::message::{Message, MessageId, MessageMetadata, IdGenerator, UuidGenerator};
//!
//! // Create a message with a UUID
//! let generator = UuidGenerator::new();
//! let msg = Message::new(42, generator.next_id());
//!
//! assert_eq!(*msg.payload(), 42);
//! ```

use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A unique identifier for messages.
///
/// Message IDs can be generated using various strategies:
/// - UUID: Globally unique, good for distributed systems
/// - Sequence: Monotonically increasing, good for ordered processing
/// - Custom: User-provided identifier (e.g., from source system)
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Default)]
pub struct MessageMetadata {
  /// When the message was created (as Duration since UNIX_EPOCH).
  pub timestamp: Option<Duration>,

  /// The source of the message (e.g., topic, file, etc.).
  pub source: Option<String>,

  /// Partition or shard information.
  pub partition: Option<u32>,

  /// Offset within the partition/source.
  pub offset: Option<u64>,

  /// User-defined key for routing/grouping.
  pub key: Option<String>,

  /// Additional headers/attributes.
  pub headers: Vec<(String, String)>,
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
  #[must_use]
  pub fn source(mut self, source: impl Into<String>) -> Self {
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
  #[must_use]
  pub fn key(mut self, key: impl Into<String>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Add a header.
  #[must_use]
  pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.headers.push((name.into(), value.into()));
    self
  }

  /// Get a header by name.
  #[must_use]
  pub fn get_header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(k, _)| k == name)
      .map(|(_, v)| v.as_str())
  }
}

/// A message envelope that wraps a payload with an ID and metadata.
///
/// This is the primary type for exactly-once processing. It provides:
/// - A unique identifier for deduplication
/// - Metadata for tracking and routing
/// - The actual payload
///
/// # Type Parameters
///
/// - `T`: The payload type
///
/// # Example
///
/// ```rust
/// use streamweave::message::{Message, MessageId, MessageMetadata};
///
/// // Create a simple message
/// let msg = Message::new(42, MessageId::new_uuid());
///
/// // Create a message with metadata
/// let msg = Message::with_metadata(
///     "hello",
///     MessageId::new_sequence(1),
///     MessageMetadata::with_timestamp_now().source("my-source")
/// );
/// ```
#[derive(Clone, Debug)]
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
pub trait IdGenerator: Send + Sync {
  /// Generate the next message ID.
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_message_id_uuid() {
    let id = MessageId::new_uuid();
    assert!(id.is_uuid());
    assert!(!id.is_sequence());
    assert!(!id.is_custom());
    assert!(!id.is_content_hash());
  }

  #[test]
  fn test_message_id_sequence() {
    let id = MessageId::new_sequence(42);
    assert!(id.is_sequence());
    assert!(!id.is_uuid());

    if let MessageId::Sequence(seq) = id {
      assert_eq!(seq, 42);
    } else {
      panic!("Expected sequence ID");
    }
  }

  #[test]
  fn test_message_id_custom() {
    let id = MessageId::new_custom("my-custom-id");
    assert!(id.is_custom());

    if let MessageId::Custom(s) = id {
      assert_eq!(s, "my-custom-id");
    } else {
      panic!("Expected custom ID");
    }
  }

  #[test]
  fn test_message_id_content_hash() {
    let id1 = MessageId::from_content(b"hello");
    let id2 = MessageId::from_content(b"hello");
    let id3 = MessageId::from_content(b"world");

    assert!(id1.is_content_hash());
    assert_eq!(id1, id2); // Same content = same hash
    assert_ne!(id1, id3); // Different content = different hash
  }

  #[test]
  fn test_message_id_display() {
    let uuid = MessageId::Uuid(0x12345678_1234_4567_89AB_CDEF01234567);
    let formatted = format!("{}", uuid);
    assert!(formatted.contains("-")); // UUID format has dashes

    let seq = MessageId::new_sequence(42);
    assert_eq!(format!("{}", seq), "seq:42");

    let custom = MessageId::new_custom("test");
    assert_eq!(format!("{}", custom), "custom:test");

    let hash = MessageId::ContentHash(0xDEADBEEF);
    assert!(format!("{}", hash).starts_with("hash:"));
  }

  #[test]
  fn test_message_creation() {
    let msg = Message::new(42, MessageId::new_sequence(1));
    assert_eq!(*msg.payload(), 42);
    assert!(msg.id().is_sequence());
  }

  #[test]
  fn test_message_with_metadata() {
    let metadata = MessageMetadata::with_timestamp_now()
      .source("test-source")
      .partition(3);

    let msg = Message::with_metadata("hello", MessageId::new_uuid(), metadata);

    assert_eq!(*msg.payload(), "hello");
    assert_eq!(msg.metadata().source, Some("test-source".to_string()));
    assert_eq!(msg.metadata().partition, Some(3));
  }

  #[test]
  fn test_message_map() {
    let msg = Message::new(42, MessageId::new_sequence(1));
    let mapped = msg.map(|x| x * 2);

    assert_eq!(*mapped.payload(), 84);
    assert_eq!(*mapped.id(), MessageId::new_sequence(1)); // ID preserved
  }

  #[test]
  fn test_message_into_parts() {
    let msg = Message::new("test", MessageId::new_sequence(5));
    let (id, payload, _metadata) = msg.into_parts();

    assert_eq!(id, MessageId::new_sequence(5));
    assert_eq!(payload, "test");
  }

  #[test]
  fn test_uuid_generator() {
    let generator = UuidGenerator::new();
    let id1 = generator.next_id();
    let id2 = generator.next_id();

    assert!(id1.is_uuid());
    assert!(id2.is_uuid());
    assert_ne!(id1, id2); // UUIDs should be unique
  }

  #[test]
  fn test_sequence_generator() {
    let generator = SequenceGenerator::new();
    let id1 = generator.next_id();
    let id2 = generator.next_id();
    let id3 = generator.next_id();

    assert_eq!(id1, MessageId::Sequence(0));
    assert_eq!(id2, MessageId::Sequence(1));
    assert_eq!(id3, MessageId::Sequence(2));
  }

  #[test]
  fn test_sequence_generator_starting_at() {
    let generator = SequenceGenerator::starting_at(100);
    let id = generator.next_id();
    assert_eq!(id, MessageId::Sequence(100));
  }

  #[test]
  fn test_sequence_generator_reset() {
    let generator = SequenceGenerator::starting_at(50);
    generator.next_id();
    generator.next_id();
    assert_eq!(generator.current(), 52);

    generator.reset();
    assert_eq!(generator.current(), 0);

    generator.reset_to(1000);
    assert_eq!(generator.current(), 1000);
  }

  #[test]
  fn test_metadata_builder() {
    let metadata = MessageMetadata::new()
      .source("my-source")
      .partition(5)
      .offset(100)
      .key("my-key")
      .header("content-type", "application/json");

    assert_eq!(metadata.source, Some("my-source".to_string()));
    assert_eq!(metadata.partition, Some(5));
    assert_eq!(metadata.offset, Some(100));
    assert_eq!(metadata.key, Some("my-key".to_string()));
    assert_eq!(
      metadata.get_header("content-type"),
      Some("application/json")
    );
    assert_eq!(metadata.get_header("non-existent"), None);
  }

  #[test]
  fn test_shared_generators() {
    let uuid_gen = uuid_generator();
    let seq_gen = sequence_generator();
    let seq_gen_from = sequence_generator_from(1000);

    assert!(uuid_gen.next_id().is_uuid());
    assert_eq!(seq_gen.next_id(), MessageId::Sequence(0));
    assert_eq!(seq_gen_from.next_id(), MessageId::Sequence(1000));
  }

  #[test]
  fn test_message_equality() {
    let id = MessageId::new_sequence(1);
    let msg1 = Message::new(42, id.clone());
    let msg2 = Message::new(42, id.clone());
    let msg3 = Message::new(43, id);

    assert_eq!(msg1, msg2);
    assert_ne!(msg1, msg3);
  }

  #[test]
  fn test_concurrent_sequence_generator() {
    use std::sync::Arc;
    use std::thread;

    let generator = Arc::new(SequenceGenerator::new());
    let mut handles = vec![];

    for _ in 0..10 {
      let generator_clone = Arc::clone(&generator);
      handles.push(thread::spawn(move || {
        let mut ids = vec![];
        for _ in 0..100 {
          ids.push(generator_clone.next_id());
        }
        ids
      }));
    }

    let mut all_ids: Vec<MessageId> = handles
      .into_iter()
      .flat_map(|h| h.join().unwrap())
      .collect();

    all_ids.sort_by_key(|id| {
      if let MessageId::Sequence(seq) = id {
        *seq
      } else {
        panic!("Expected sequence ID")
      }
    });

    // Check all IDs are unique (sequence 0..1000)
    for (i, id) in all_ids.iter().enumerate() {
      assert_eq!(*id, MessageId::Sequence(i as u64));
    }
  }
}
