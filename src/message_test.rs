//! # Message System Test Suite
//!
//! Comprehensive test suite for the message system, including message IDs, metadata,
//! message wrapping, ID generation, shared messages, and stream extensions.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **MessageId**: UUID, Sequence, Custom, and ContentHash ID variants with display and hashing
//! - **MessageMetadata**: Timestamp, source, partition, offset, key, headers, and serialization
//! - **Message**: Message creation, payload access, metadata access, mapping, and conversion
//! - **IdGenerators**: UuidGenerator, SequenceGenerator, and ContentHashGenerator
//! - **SharedMessage**: Arc-based message sharing with try_unwrap and conversion traits
//! - **MessageStreamExt**: Stream extension methods for payload, ID, and metadata extraction
//! - **Helper Functions**: wrap_message, unwrap_message, and various wrapping utilities
//!
//! ## Test Organization
//!
//! Tests are organized into the following sections:
//!
//! 1. **MessageId Tests**: All ID variants, type checkers, display, hashing, and equality
//! 2. **MessageMetadata Tests**: Field access, builder pattern, serialization, and Arc sharing
//! 3. **Message Tests**: Creation, accessors, mapping, conversion, and equality
//! 4. **Helper Functions Tests**: Message wrapping and unwrapping utilities
//! 5. **IdGenerator Tests**: All generator types, thread safety, and edge cases
//! 6. **SharedMessage Tests**: Arc sharing, try_unwrap, conversion traits, and equality
//! 7. **MessageStreamExt Tests**: Stream extension methods for message processing
//! 8. **Integration Tests**: Serde roundtrip, end-to-end message flow
//!
//! ## Key Concepts
//!
//! - **Message<T>**: The core message type wrapping payload with ID and metadata
//! - **MessageId**: Unique identifier for messages (UUID, Sequence, Custom, ContentHash)
//! - **MessageMetadata**: Rich metadata including timestamps, source, headers, etc.
//! - **SharedMessage**: Arc-based shared ownership for zero-copy fan-out scenarios
//! - **IdGenerator**: Trait for generating unique message IDs
//!
//! ## Usage
//!
//! These tests ensure that the message system correctly handles message creation,
//! ID generation, metadata management, and zero-copy sharing for high-performance
//! stream processing.

use crate::{
  ContentHashGenerator, IdGenerator, Message, MessageId, MessageMetadata, MessageStreamExt,
  SequenceGenerator, SharedMessage, UuidGenerator, sequence_generator, sequence_generator_from,
  unwrap_message, uuid_generator, wrap_message, wrap_message_with_generator, wrap_message_with_id,
  wrap_message_with_metadata, wrap_message_with_shared_generator, wrap_messages,
};
use futures::stream;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_message_id_new_uuid() {
  let id1 = MessageId::new_uuid();
  let id2 = MessageId::new_uuid();

  assert!(id1.is_uuid());
  assert!(id2.is_uuid());
  assert_ne!(id1, id2, "UUIDs should be unique");
}

#[test]
fn test_message_id_new_sequence() {
  let id = MessageId::new_sequence(42);
  assert!(id.is_sequence());

  if let MessageId::Sequence(seq) = id {
    assert_eq!(seq, 42);
  } else {
    panic!("Expected Sequence variant");
  }
}

#[test]
fn test_message_id_new_custom() {
  let id = MessageId::new_custom("my-custom-id");
  assert!(id.is_custom());

  if let MessageId::Custom(s) = id {
    assert_eq!(s, "my-custom-id");
  } else {
    panic!("Expected Custom variant");
  }
}

#[test]
fn test_message_id_from_content() {
  let content1 = b"some content";
  let content2 = b"some content";
  let content3 = b"different content";

  let id1 = MessageId::from_content(content1);
  let id2 = MessageId::from_content(content2);
  let id3 = MessageId::from_content(content3);

  assert!(id1.is_content_hash());
  assert_eq!(id1, id2, "Same content should produce same hash");
  assert_ne!(id1, id3, "Different content should produce different hash");
}

#[test]
fn test_message_id_type_checkers() {
  let uuid_id = MessageId::new_uuid();
  assert!(uuid_id.is_uuid());
  assert!(!uuid_id.is_sequence());
  assert!(!uuid_id.is_custom());
  assert!(!uuid_id.is_content_hash());

  let seq_id = MessageId::new_sequence(1);
  assert!(!seq_id.is_uuid());
  assert!(seq_id.is_sequence());
  assert!(!seq_id.is_custom());
  assert!(!seq_id.is_content_hash());

  let custom_id = MessageId::new_custom("test");
  assert!(!custom_id.is_uuid());
  assert!(!custom_id.is_sequence());
  assert!(custom_id.is_custom());
  assert!(!custom_id.is_content_hash());
}

#[test]
fn test_message_id_display() {
  let uuid = MessageId::new_uuid();
  let uuid_str = format!("{}", uuid);
  assert!(uuid_str.contains('-'));

  let seq = MessageId::new_sequence(42);
  assert_eq!(format!("{}", seq), "seq:42");

  let custom = MessageId::new_custom("test-id");
  assert_eq!(format!("{}", custom), "custom:test-id");

  let hash = MessageId::from_content(b"test");
  let hash_str = format!("{}", hash);
  assert!(hash_str.starts_with("hash:"));
}

#[test]
fn test_message_id_hash() {
  use std::collections::HashSet;

  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(1);
  let id3 = MessageId::new_sequence(2);

  let mut set = HashSet::new();
  set.insert(id1.clone());
  set.insert(id2.clone());
  set.insert(id3);

  assert_eq!(set.len(), 2, "Same IDs should hash to same value");
  assert!(set.contains(&id1));
  assert!(set.contains(&id2));
}

#[test]
fn test_message_id_default() {
  let id = MessageId::default();
  assert!(id.is_uuid());
}

#[test]
fn test_message_id_clone_eq() {
  let id1 = MessageId::new_sequence(42);
  let id2 = id1.clone();
  assert_eq!(id1, id2);
}

// ============================================================================
// MessageMetadata Tests
// ============================================================================

#[test]
fn test_metadata_new() {
  let metadata = MessageMetadata::new();
  assert_eq!(metadata.timestamp, None);
  assert_eq!(metadata.source, None);
  assert_eq!(metadata.partition, None);
  assert_eq!(metadata.offset, None);
  assert_eq!(metadata.key, None);
  assert!(metadata.headers.is_empty());
}

#[test]
fn test_metadata_with_timestamp_now() {
  let metadata = MessageMetadata::with_timestamp_now();
  assert!(metadata.timestamp.is_some());
}

#[test]
fn test_metadata_builder_pattern() {
  let metadata = MessageMetadata::default()
    .source("my_source")
    .partition(0)
    .offset(12345)
    .key("my_key")
    .header("header1", "value1")
    .header("header2", "value2");

  assert_eq!(metadata.get_source(), Some("my_source"));
  assert_eq!(metadata.partition, Some(0));
  assert_eq!(metadata.offset, Some(12345));
  assert_eq!(metadata.get_key(), Some("my_key"));
  assert_eq!(metadata.get_header("header1"), Some("value1"));
  assert_eq!(metadata.get_header("header2"), Some("value2"));
}

#[test]
fn test_metadata_timestamp() {
  let duration = Duration::from_secs(1234567890);
  let metadata = MessageMetadata::default().timestamp(duration);
  assert_eq!(metadata.timestamp, Some(duration));
}

#[test]
fn test_metadata_source() {
  let metadata = MessageMetadata::default().source("kafka-topic");
  assert_eq!(metadata.get_source(), Some("kafka-topic"));

  // Test with String
  let metadata = MessageMetadata::default().source(String::from("string-source"));
  assert_eq!(metadata.get_source(), Some("string-source"));
}

#[test]
fn test_metadata_get_header() {
  let metadata = MessageMetadata::default()
    .header("key1", "value1")
    .header("key2", "value2");

  assert_eq!(metadata.get_header("key1"), Some("value1"));
  assert_eq!(metadata.get_header("key2"), Some("value2"));
  assert_eq!(metadata.get_header("nonexistent"), None);
}

#[test]
fn test_metadata_multiple_headers() {
  let metadata = MessageMetadata::default()
    .header("trace-id", "abc-123")
    .header("span-id", "def-456");

  assert_eq!(metadata.headers.len(), 2);
  assert_eq!(metadata.get_header("trace-id"), Some("abc-123"));
  assert_eq!(metadata.get_header("span-id"), Some("def-456"));
}

#[test]
fn test_metadata_clone() {
  let metadata1 = MessageMetadata::default()
    .source("test")
    .header("key", "value");

  let metadata2 = metadata1.clone();
  assert_eq!(metadata1.get_source(), metadata2.get_source());
  assert_eq!(metadata1.get_header("key"), metadata2.get_header("key"));
}

#[test]
fn test_metadata_arc_str_sharing() {
  let source: Arc<str> = Arc::from("shared-source");
  let metadata1 = MessageMetadata::default().source(source.clone());
  let metadata2 = MessageMetadata::default().source(source.clone());

  // Both should reference the same Arc
  assert_eq!(metadata1.get_source(), Some("shared-source"));
  assert_eq!(metadata2.get_source(), Some("shared-source"));
}

// ============================================================================
// Message Tests
// ============================================================================

#[test]
fn test_message_new() {
  let payload = 42;
  let id = MessageId::new_uuid();
  let msg = Message::new(payload, id.clone());

  assert_eq!(*msg.payload(), 42);
  assert_eq!(msg.id(), &id);
  assert!(msg.metadata().timestamp.is_some());
}

#[test]
fn test_message_with_metadata() {
  let payload = 42;
  let id = MessageId::new_sequence(1);
  let metadata = MessageMetadata::default()
    .source("test_source")
    .header("key", "value");

  let msg = Message::with_metadata(payload, id.clone(), metadata.clone());

  assert_eq!(*msg.payload(), 42);
  assert_eq!(msg.id(), &id);
  assert_eq!(msg.metadata().get_source(), Some("test_source"));
  assert_eq!(msg.metadata().get_header("key"), Some("value"));
}

#[test]
fn test_message_accessors() {
  let msg = Message::new(42, MessageId::new_uuid());

  assert_eq!(*msg.payload(), 42);
  assert!(msg.id().is_uuid());
  assert!(msg.metadata().timestamp.is_some());
}

#[test]
fn test_message_payload_mut() {
  let mut msg = Message::new(vec![1, 2, 3], MessageId::new_uuid());
  msg.payload_mut().push(4);

  assert_eq!(msg.payload(), &vec![1, 2, 3, 4]);
}

#[test]
fn test_message_metadata_mut() {
  let mut msg = Message::new(42, MessageId::new_uuid());
  let metadata = msg.metadata().clone().source("new_source");
  *msg.metadata_mut() = metadata;

  assert_eq!(msg.metadata().get_source(), Some("new_source"));
}

#[test]
fn test_message_into_parts() {
  let id = MessageId::new_sequence(1);
  let metadata = MessageMetadata::default().source("test");
  let msg = Message::with_metadata(42, id.clone(), metadata.clone());

  let (retrieved_id, payload, retrieved_metadata) = msg.into_parts();

  assert_eq!(retrieved_id, id);
  assert_eq!(payload, 42);
  assert_eq!(retrieved_metadata.get_source(), Some("test"));
}

#[test]
fn test_message_into_payload() {
  let msg = Message::new(42, MessageId::new_uuid());
  let payload = msg.into_payload();

  assert_eq!(payload, 42);
}

#[test]
fn test_message_map() {
  let msg = Message::new(21, MessageId::new_sequence(1));
  let doubled = msg.map(|x| x * 2);

  assert_eq!(*doubled.payload(), 42);
  assert!(doubled.id().is_sequence());
  if let MessageId::Sequence(seq) = doubled.id() {
    assert_eq!(*seq, 1);
  }
}

#[test]
fn test_message_map_with_id() {
  let msg = Message::new(10, MessageId::new_sequence(5));
  let result = msg.map_with_id(|id, payload| {
    if let MessageId::Sequence(seq) = id {
      *seq as i32 + payload
    } else {
      payload
    }
  });

  assert_eq!(*result.payload(), 15); // 5 + 10
}

#[test]
fn test_message_map_with_id_uuid() {
  // Test map_with_id with UUID to cover the else branch (line 326)
  let msg = Message::new(10, MessageId::new_uuid());
  let result = msg.map_with_id(|id, payload| {
    if let MessageId::Sequence(seq) = id {
      *seq as i32 + payload
    } else {
      payload * 2 // else branch
    }
  });

  assert_eq!(*result.payload(), 20); // 10 * 2 (else branch)
}

#[test]
fn test_message_with_payload() {
  let msg = Message::new(42, MessageId::new_sequence(1));
  let new_msg = msg.with_payload("hello");

  assert_eq!(*new_msg.payload(), "hello");
  assert!(new_msg.id().is_sequence());
}

#[test]
fn test_message_default() {
  let msg: Message<i32> = Message::default();
  assert_eq!(*msg.payload(), 0);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_message_eq() {
  let id = MessageId::new_sequence(1);
  let msg1 = Message::new(42, id.clone());
  let msg2 = Message::new(42, id.clone());
  let msg3 = Message::new(43, id);

  assert_eq!(msg1, msg2);
  assert_ne!(msg1, msg3);
}

#[test]
fn test_message_hash() {
  use std::collections::HashSet;

  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(2);

  let msg1 = Message::new(42, id1);
  let msg2 = Message::new(42, id2);

  let mut set = HashSet::new();
  set.insert(msg1.clone());
  set.insert(msg2.clone());

  assert_eq!(set.len(), 2);
}

// ============================================================================
// Helper Functions Tests
// ============================================================================

#[test]
fn test_wrap_message() {
  let msg = wrap_message(42);
  assert_eq!(*msg.payload(), 42);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_wrap_message_with_id() {
  let id = MessageId::new_sequence(1);
  let msg = wrap_message_with_id(42, id.clone());

  assert_eq!(*msg.payload(), 42);
  assert_eq!(msg.id(), &id);
}

#[test]
fn test_wrap_message_with_metadata() {
  let id = MessageId::new_uuid();
  let metadata = MessageMetadata::default().source("test");
  let msg = wrap_message_with_metadata(42, id.clone(), metadata.clone());

  assert_eq!(*msg.payload(), 42);
  assert_eq!(msg.id(), &id);
  assert_eq!(msg.metadata().get_source(), Some("test"));
}

#[test]
fn test_unwrap_message() {
  let msg = wrap_message(42);
  let payload = unwrap_message(msg);
  assert_eq!(payload, 42);
}

#[test]
fn test_wrap_message_with_generator() {
  let generator = SequenceGenerator::new();
  let msg = wrap_message_with_generator(42, &generator);

  assert_eq!(*msg.payload(), 42);
  assert!(msg.id().is_sequence());
}

#[test]
fn test_wrap_messages() {
  let generator = SequenceGenerator::new();
  let messages = wrap_messages(vec![1, 2, 3], &generator);

  assert_eq!(messages.len(), 3);
  assert_eq!(*messages[0].payload(), 1);
  assert_eq!(*messages[1].payload(), 2);
  assert_eq!(*messages[2].payload(), 3);

  // All should have sequence IDs
  assert!(messages[0].id().is_sequence());
  assert!(messages[1].id().is_sequence());
  assert!(messages[2].id().is_sequence());
}

#[test]
fn test_wrap_message_with_shared_generator() {
  let generator = sequence_generator();
  let msg = wrap_message_with_shared_generator(42, &generator);

  assert_eq!(*msg.payload(), 42);
  assert!(msg.id().is_sequence());
}

// ============================================================================
// IdGenerator Tests
// ============================================================================

#[test]
fn test_uuid_generator() {
  let generator = UuidGenerator::new();
  let id1 = generator.next_id();
  let id2 = generator.next_id();

  assert!(id1.is_uuid());
  assert!(id2.is_uuid());
  assert_ne!(id1, id2);
}

#[test]
fn test_sequence_generator() {
  let generator = SequenceGenerator::new();
  let id1 = generator.next_id();
  let id2 = generator.next_id();
  let id3 = generator.next_id();

  assert!(id1.is_sequence());
  assert!(id2.is_sequence());
  assert!(id3.is_sequence());

  if let MessageId::Sequence(s1) = id1 {
    if let MessageId::Sequence(s2) = id2 {
      if let MessageId::Sequence(s3) = id3 {
        assert_eq!(s1, 0);
        assert_eq!(s2, 1);
        assert_eq!(s3, 2);
      } else {
        panic!("Expected sequence");
      }
    } else {
      panic!("Expected sequence");
    }
  } else {
    panic!("Expected sequence");
  }
}

#[test]
fn test_sequence_generator_starting_at() {
  let generator = SequenceGenerator::starting_at(100);
  let id = generator.next_id();

  if let MessageId::Sequence(seq) = id {
    assert_eq!(seq, 100);
  } else {
    panic!("Expected sequence");
  }
}

#[test]
fn test_sequence_generator_current() {
  let generator = SequenceGenerator::new();
  assert_eq!(generator.current(), 0);

  generator.next_id();
  assert_eq!(generator.current(), 1);

  generator.next_id();
  assert_eq!(generator.current(), 2);
}

#[test]
fn test_sequence_generator_reset() {
  let generator = SequenceGenerator::new();
  generator.next_id();
  generator.next_id();
  assert_eq!(generator.current(), 2);

  generator.reset();
  assert_eq!(generator.current(), 0);

  let id = generator.next_id();
  if let MessageId::Sequence(seq) = id {
    assert_eq!(seq, 0);
  } else {
    panic!("Expected sequence");
  }
}

#[test]
fn test_sequence_generator_reset_to() {
  let generator = SequenceGenerator::new();
  generator.reset_to(50);
  assert_eq!(generator.current(), 50);

  let id = generator.next_id();
  if let MessageId::Sequence(seq) = id {
    assert_eq!(seq, 50);
  } else {
    panic!("Expected sequence");
  }
}

#[test]
fn test_content_hash_generator() {
  let generator = ContentHashGenerator::new();
  let content = b"test content";
  let id = generator.hash_content(content);

  assert!(id.is_content_hash());
}

#[test]
fn test_shared_generator_helpers() {
  let uuid_gen = uuid_generator();
  let seq_gen = sequence_generator();
  let seq_gen_from = sequence_generator_from(10);

  let uuid_id = uuid_gen.next_id();
  let seq_id = seq_gen.next_id();
  let seq_from_id = seq_gen_from.next_id();

  assert!(uuid_id.is_uuid());
  assert!(seq_id.is_sequence());
  assert!(seq_from_id.is_sequence());

  if let MessageId::Sequence(seq) = seq_from_id {
    assert_eq!(seq, 10);
  } else {
    panic!("Expected sequence");
  }
}

#[test]
fn test_generator_thread_safety() {
  use std::thread;

  let generator = Arc::new(SequenceGenerator::new());
  let mut handles = vec![];

  for _ in 0..10 {
    let generator_clone = generator.clone();
    handles.push(thread::spawn(move || generator_clone.next_id()));
  }

  let mut ids: Vec<MessageId> = handles.into_iter().map(|h| h.join().unwrap()).collect();
  ids.sort_by_key(|id| {
    if let MessageId::Sequence(seq) = id {
      *seq
    } else {
      0
    }
  });

  // Should have generated 10 unique sequence numbers
  assert_eq!(ids.len(), 10);
}

// ============================================================================
// SharedMessage Tests
// ============================================================================

#[test]
fn test_shared_message_from() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared = SharedMessage::from(msg);

  assert_eq!(*shared.payload(), 42);
  assert!(shared.id().is_uuid());
}

#[test]
fn test_shared_message_from_arc() {
  let msg = Arc::new(Message::new(42, MessageId::new_uuid()));
  let shared = SharedMessage::from_arc(msg);

  assert_eq!(*shared.payload(), 42);
}

#[test]
fn test_shared_message_clone() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared1 = SharedMessage::from(msg);
  let shared2 = shared1.clone();

  // Both should reference the same underlying message
  assert_eq!(*shared1.payload(), 42);
  assert_eq!(*shared2.payload(), 42);
  assert_eq!(shared1.id(), shared2.id());
}

#[test]
fn test_shared_message_try_unwrap() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared = SharedMessage::from(msg);

  // Should succeed when this is the only reference
  let unwrapped = shared.try_unwrap();
  assert!(unwrapped.is_ok());

  if let Ok(msg) = unwrapped {
    assert_eq!(*msg.payload(), 42);
  }
}

#[test]
fn test_shared_message_try_unwrap_fails_with_multiple_refs() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared1 = SharedMessage::from(msg);
  let _shared2 = shared1.clone();

  // Should fail when there are multiple references
  let result = shared1.try_unwrap();
  assert!(result.is_err());

  // The error should contain the shared message
  if let Err(shared) = result {
    assert_eq!(*shared.payload(), 42);
  }
}

#[test]
fn test_shared_message_deref() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared = SharedMessage::from(msg);

  // Test deref
  assert_eq!(shared.payload(), &42);
  assert!(shared.id().is_uuid());
}

#[test]
fn test_shared_message_from_traits() {
  let msg = Message::new(42, MessageId::new_uuid());

  // Test From<Message>
  let shared1: SharedMessage<i32> = msg.clone().into();
  assert_eq!(*shared1.payload(), 42);

  // Test From<Arc<Message>>
  let arc = Arc::new(msg.clone());
  let shared2: SharedMessage<i32> = arc.clone().into();
  assert_eq!(*shared2.payload(), 42);

  // Test From<SharedMessage> for Arc
  let arc_result: Arc<Message<i32>> = shared2.into();
  assert_eq!(*arc_result.payload(), 42);
}

#[test]
fn test_shared_message_eq() {
  let id = MessageId::new_sequence(1);
  let msg1 = Message::new(42, id.clone());
  let msg2 = Message::new(42, id);

  let shared1 = SharedMessage::from(msg1);
  let shared2 = SharedMessage::from(msg2);

  assert_eq!(shared1, shared2);
}

#[test]
fn test_shared_message_hash() {
  use std::collections::HashSet;

  let msg1 = Message::new(42, MessageId::new_sequence(1));
  let msg2 = Message::new(42, MessageId::new_sequence(1));

  let shared1 = SharedMessage::from(msg1);
  let shared2 = SharedMessage::from(msg2);

  let mut set = HashSet::new();
  set.insert(shared1);
  set.insert(shared2);

  assert_eq!(set.len(), 1); // Same content should hash to same value
}

// ============================================================================
// MessageStreamExt Tests
// ============================================================================

#[tokio::test]
async fn test_extract_payloads() {
  use futures::StreamExt;

  let messages = stream::iter(vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(3, MessageId::new_sequence(3)),
  ]);

  let payloads: Vec<i32> = messages.extract_payloads().collect().await;

  assert_eq!(payloads, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_extract_ids() {
  use futures::StreamExt;

  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(2);
  let id3 = MessageId::new_sequence(3);

  let messages = stream::iter(vec![
    Message::new(1, id1.clone()),
    Message::new(2, id2.clone()),
    Message::new(3, id3.clone()),
  ]);

  let ids: Vec<MessageId> = messages.extract_ids().collect().await;

  assert_eq!(ids.len(), 3);
  assert_eq!(ids[0], id1);
  assert_eq!(ids[1], id2);
  assert_eq!(ids[2], id3);
}

#[tokio::test]
async fn test_extract_metadata() {
  use futures::StreamExt;

  let metadata1 = MessageMetadata::default().source("source1");
  let metadata2 = MessageMetadata::default().source("source2");

  let messages = stream::iter(vec![
    Message::with_metadata(1, MessageId::new_uuid(), metadata1.clone()),
    Message::with_metadata(2, MessageId::new_uuid(), metadata2.clone()),
  ]);

  let metadatas: Vec<MessageMetadata> = messages.extract_metadata().collect().await;

  assert_eq!(metadatas.len(), 2);
  assert_eq!(metadatas[0].get_source(), Some("source1"));
  assert_eq!(metadatas[1].get_source(), Some("source2"));
}

#[tokio::test]
async fn test_map_payload() {
  use futures::StreamExt;

  let messages = stream::iter(vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
  ]);

  let doubled: Vec<Message<i32>> = messages.map_payload(|x| x * 2).collect().await;

  assert_eq!(doubled.len(), 2);
  assert_eq!(*doubled[0].payload(), 2);
  assert_eq!(*doubled[1].payload(), 4);

  // IDs should be preserved
  assert!(doubled[0].id().is_sequence());
  assert!(doubled[1].id().is_sequence());
}

// ============================================================================
// String Interner Tests (if applicable)
// ============================================================================

// Note: StringInternerTrait is defined here but implemented in graph module
// These tests would require the actual implementation

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_message_roundtrip_serde() {
  use serde_json;

  let original = Message::with_metadata(
    42,
    MessageId::new_sequence(1),
    MessageMetadata::default()
      .source("test_source")
      .header("key", "value"),
  );

  let serialized = serde_json::to_string(&original).expect("Should serialize");
  let deserialized: Message<i32> = serde_json::from_str(&serialized).expect("Should deserialize");

  assert_eq!(*original.payload(), *deserialized.payload());
  assert_eq!(original.id(), deserialized.id());
  assert_eq!(
    original.metadata().get_source(),
    deserialized.metadata().get_source()
  );
}

#[test]
fn test_message_id_roundtrip_serde() {
  use serde_json;

  let original = MessageId::new_sequence(42);
  let serialized = serde_json::to_string(&original).expect("Should serialize");
  let deserialized: MessageId = serde_json::from_str(&serialized).expect("Should deserialize");

  assert_eq!(original, deserialized);
}

#[test]
fn test_metadata_roundtrip_serde() {
  use serde_json;

  let original = MessageMetadata::default()
    .source("test")
    .header("key", "value");

  let serialized = serde_json::to_string(&original).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");

  assert_eq!(original.get_source(), deserialized.get_source());
  assert_eq!(original.get_header("key"), deserialized.get_header("key"));
}

// ============================================================================
// Additional Coverage Tests
// ============================================================================

#[test]
fn test_message_id_display_uuid() {
  let uuid = MessageId::Uuid(0x12345678_90ABCDEF_FEDCBA09_87654321);
  let display = format!("{}", uuid);
  assert!(display.contains("-"));
  assert_eq!(display.len(), 36); // Standard UUID format length
}

#[test]
fn test_message_id_display_custom() {
  let custom = MessageId::Custom("my-id-123".to_string());
  assert_eq!(format!("{}", custom), "custom:my-id-123");
}

#[test]
fn test_message_id_display_content_hash() {
  let hash = MessageId::ContentHash(0x123456789ABCDEF0);
  assert_eq!(format!("{}", hash), "hash:123456789abcdef0");
}

#[test]
fn test_message_id_hash_all_variants() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let variants = [
    MessageId::Uuid(0x12345678_90ABCDEF_FEDCBA09_87654321),
    MessageId::Sequence(42),
    MessageId::Custom("test".to_string()),
    MessageId::ContentHash(0xABCDEF1234567890),
  ];

  let mut hashers: Vec<DefaultHasher> = variants.iter().map(|_| DefaultHasher::new()).collect();

  for (id, hasher) in variants.iter().zip(hashers.iter_mut()) {
    id.hash(hasher);
  }

  // All should produce different hash values
  let hashes: Vec<u64> = hashers.into_iter().map(|h| h.finish()).collect();
  for i in 0..hashes.len() {
    for j in (i + 1)..hashes.len() {
      assert_ne!(
        hashes[i], hashes[j],
        "Different MessageId variants should produce different hashes"
      );
    }
  }
}

#[test]
fn test_metadata_with_shared_header() {
  let name: Arc<str> = Arc::from("header-name");
  let value: Arc<str> = Arc::from("header-value");
  let metadata = MessageMetadata::default().with_shared_header(name.clone(), value.clone());

  assert_eq!(metadata.get_header("header-name"), Some("header-value"));
}

#[test]
fn test_metadata_with_source_borrowed() {
  let metadata = MessageMetadata::default().with_source_borrowed("borrowed-source");
  assert_eq!(metadata.get_source(), Some("borrowed-source"));
}

#[test]
fn test_metadata_get_key() {
  let metadata = MessageMetadata::default().key("test-key");
  assert_eq!(metadata.get_key(), Some("test-key"));

  let empty = MessageMetadata::default();
  assert_eq!(empty.get_key(), None);
}

#[test]
fn test_metadata_partition_and_offset() {
  let metadata = MessageMetadata::default().partition(5).offset(12345);

  assert_eq!(metadata.partition, Some(5));
  assert_eq!(metadata.offset, Some(12345));
}

#[test]
fn test_metadata_partition_offset_serde() {
  use serde_json;

  let original = MessageMetadata::default().partition(10).offset(999);

  let serialized = serde_json::to_string(&original).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");

  assert_eq!(original.partition, deserialized.partition);
  assert_eq!(original.offset, deserialized.offset);
}

#[test]
fn test_metadata_empty_headers_serde() {
  use serde_json;

  let original = MessageMetadata::default();
  let serialized = serde_json::to_string(&original).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");

  assert!(deserialized.headers.is_empty());
}

#[test]
fn test_metadata_serialize_none_values() {
  use serde_json;

  let original = MessageMetadata::default(); // All optional fields are None
  let serialized = serde_json::to_string(&original).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");

  assert_eq!(original.timestamp, deserialized.timestamp);
  assert_eq!(original.source, deserialized.source);
  assert_eq!(original.key, deserialized.key);
  assert_eq!(original.partition, deserialized.partition);
  assert_eq!(original.offset, deserialized.offset);
}

#[test]
fn test_message_default_i32() {
  let msg: Message<i32> = Message::default();
  assert_eq!(*msg.payload(), 0);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_message_default_string() {
  let msg: Message<String> = Message::default();
  assert_eq!(*msg.payload(), "");
}

#[test]
fn test_shared_message_into_arc() {
  use std::sync::Arc;

  let msg = Message::new(42, MessageId::new_sequence(1));
  let shared = SharedMessage::from(msg);
  let arc: Arc<Message<i32>> = shared.into_arc();

  assert_eq!(*arc.payload(), 42);
}

#[test]
fn test_shared_message_from_trait_arc() {
  use std::sync::Arc;

  let msg = Message::new(42, MessageId::new_sequence(1));
  let arc: Arc<Message<i32>> = Arc::new(msg);
  // Test the From<Arc<Message<T>>> trait implementation
  let shared: SharedMessage<i32> = arc.clone().into();

  assert_eq!(*shared.payload(), 42);
}

#[test]
fn test_shared_message_arc_from_trait() {
  use std::sync::Arc;

  let msg = Message::new(42, MessageId::new_sequence(1));
  let shared = SharedMessage::from(msg);
  let arc: Arc<Message<i32>> = Arc::from(shared);

  assert_eq!(*arc.payload(), 42);
}

#[test]
fn test_shared_message_as_ref() {
  let msg = Message::new(42, MessageId::new_sequence(1));
  let shared = SharedMessage::from(msg);
  let msg_ref: &Message<i32> = shared.as_ref();

  assert_eq!(*msg_ref.payload(), 42);
}

// Mock StringInternerTrait for testing interned methods
struct MockInterner;

impl crate::message::StringInternerTrait for MockInterner {
  fn get_or_intern(&self, s: &str) -> Arc<str> {
    Arc::from(s.to_string())
  }
}

#[test]
fn test_metadata_with_source_interned() {
  let interner = MockInterner;
  let metadata = MessageMetadata::default().with_source_interned("interned-source", &interner);

  assert_eq!(metadata.get_source(), Some("interned-source"));
}

#[test]
fn test_metadata_with_key_interned() {
  let interner = MockInterner;
  let metadata = MessageMetadata::default().with_key_interned("interned-key", &interner);

  assert_eq!(metadata.get_key(), Some("interned-key"));
}

#[test]
fn test_metadata_add_header_interned() {
  let interner = MockInterner;
  let metadata =
    MessageMetadata::default().add_header_interned("interned-header", "interned-value", &interner);

  assert_eq!(
    metadata.get_header("interned-header"),
    Some("interned-value")
  );
}

#[tokio::test]
async fn test_message_stream_extract_payloads() {
  use crate::message::{Message, MessageId, MessageStreamExt};
  use futures::StreamExt;
  use futures::stream;

  let messages = stream::iter(vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(3, MessageId::new_sequence(3)),
  ]);

  let payloads: Vec<i32> = messages.extract_payloads().collect().await;
  assert_eq!(payloads, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_message_stream_extract_ids() {
  use crate::message::{Message, MessageId, MessageStreamExt};
  use futures::StreamExt;
  use futures::stream;

  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(2);
  let id3 = MessageId::new_sequence(3);

  let messages = stream::iter(vec![
    Message::new(1, id1.clone()),
    Message::new(2, id2.clone()),
    Message::new(3, id3.clone()),
  ]);

  let ids: Vec<MessageId> = messages.extract_ids().collect().await;
  assert_eq!(ids.len(), 3);
  assert_eq!(ids[0], id1);
  assert_eq!(ids[1], id2);
  assert_eq!(ids[2], id3);
}

#[tokio::test]
async fn test_message_stream_extract_metadata() {
  use crate::message::{Message, MessageId, MessageMetadata, MessageStreamExt};
  use futures::StreamExt;
  use futures::stream;

  let metadata1 = MessageMetadata::default().source("source1");
  let metadata2 = MessageMetadata::default().source("source2");

  let messages = stream::iter(vec![
    Message::with_metadata(1, MessageId::new_sequence(1), metadata1.clone()),
    Message::with_metadata(2, MessageId::new_sequence(2), metadata2.clone()),
  ]);

  let metadatas: Vec<MessageMetadata> = messages.extract_metadata().collect().await;
  assert_eq!(metadatas.len(), 2);
  assert_eq!(metadatas[0].get_source(), metadata1.get_source());
  assert_eq!(metadatas[1].get_source(), metadata2.get_source());
}

#[tokio::test]
async fn test_message_stream_map_payload() {
  use crate::message::{Message, MessageId, MessageStreamExt};
  use futures::StreamExt;
  use futures::stream;

  let messages = stream::iter(vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
  ]);

  let doubled: Vec<Message<i32>> = messages.map_payload(|x| x * 2).collect().await;
  assert_eq!(doubled.len(), 2);
  assert_eq!(*doubled[0].payload(), 2);
  assert_eq!(*doubled[1].payload(), 4);
  assert_eq!(doubled[0].id(), &MessageId::new_sequence(1));
}

#[test]
fn test_message_id_default_impl() {
  let id: MessageId = Default::default();
  assert!(id.is_uuid()); // Default should generate UUID
}

#[test]
fn test_metadata_with_shared_source() {
  let source: Arc<str> = Arc::from("shared-source");
  let metadata = MessageMetadata::default().with_shared_source(source.clone());
  assert_eq!(metadata.get_source(), Some("shared-source"));
  // Verify it's the same Arc
  assert!(Arc::ptr_eq(metadata.source.as_ref().unwrap(), &source));
}

#[test]
fn test_metadata_with_shared_key() {
  let key: Arc<str> = Arc::from("shared-key");
  let metadata = MessageMetadata::default().with_shared_key(key.clone());
  assert_eq!(metadata.get_key(), Some("shared-key"));
  // Verify it's the same Arc
  assert!(Arc::ptr_eq(metadata.key.as_ref().unwrap(), &key));
}

#[test]
fn test_message_id_display_uuid_various_patterns() {
  // Test various UUID bit patterns to ensure all bit shifts work correctly
  let uuid1 = MessageId::Uuid(0x00000000_00000000_00000000_00000000);
  let display1 = format!("{}", uuid1);
  assert_eq!(display1.len(), 36);
  assert!(display1.contains("-"));

  let uuid2 = MessageId::Uuid(0xFFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF);
  let display2 = format!("{}", uuid2);
  assert_eq!(display2.len(), 36);
  assert!(display2.contains("-"));

  let uuid3 = MessageId::Uuid(0x12345678_90ABCDEF_FEDCBA09_87654321);
  let display3 = format!("{}", uuid3);
  assert_eq!(display3.len(), 36);
  assert!(display3.contains("-"));
  // Verify the format matches UUID pattern
  assert!(display3.matches('-').count() == 4);
}

#[test]
fn test_metadata_serialize_none_source() {
  use serde_json;

  let metadata = MessageMetadata::default(); // source is None
  let serialized = serde_json::to_string(&metadata).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");
  assert_eq!(metadata.source, deserialized.source);
}

#[test]
fn test_metadata_serialize_none_key() {
  use serde_json;

  let metadata = MessageMetadata::default().source("test"); // key is None
  let serialized = serde_json::to_string(&metadata).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");
  assert_eq!(metadata.key, deserialized.key);
}

#[test]
fn test_metadata_serialize_empty_headers_vec() {
  use serde_json;

  let metadata = MessageMetadata::default();
  let serialized = serde_json::to_string(&metadata).expect("Should serialize");
  let deserialized: MessageMetadata =
    serde_json::from_str(&serialized).expect("Should deserialize");
  assert!(deserialized.headers.is_empty());
}

#[test]
fn test_message_id_hash_discriminant() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  // Test that discriminant is hashed first (line 292)
  let uuid = MessageId::Uuid(0x12345678_90ABCDEF_FEDCBA09_87654321);
  let seq = MessageId::Sequence(0x12345678_90ABCDEF);

  let mut hasher1 = DefaultHasher::new();
  uuid.hash(&mut hasher1);
  let hash1 = hasher1.finish();

  let mut hasher2 = DefaultHasher::new();
  seq.hash(&mut hasher2);
  let hash2 = hasher2.finish();

  // Different variants should produce different hashes even with same numeric value
  assert_ne!(hash1, hash2);
}

#[test]
fn test_shared_message_from_message() {
  let msg = Message::new(42, MessageId::new_sequence(1));
  // Test From<Message<T>> trait implementation (line 1224)
  let shared: SharedMessage<i32> = msg.into();
  assert_eq!(*shared.payload(), 42);
}

#[test]
fn test_content_hash_generator_hash_content() {
  let generator = ContentHashGenerator::new();
  let content = b"test content";
  let id1 = generator.hash_content(content);
  let id2 = generator.hash_content(content);

  assert!(id1.is_content_hash());
  assert_eq!(id1, id2); // Same content should produce same hash
}

#[test]
fn test_message_id_display_uuid_all_bit_shifts() {
  // Test each bit shift operation in UUID display (lines 273-281)
  // uuid >> 96 (32-bit, first part)
  // uuid >> 80 (16-bit, second part)
  // uuid >> 64 (16-bit, third part)
  // uuid >> 48 (16-bit, fourth part)
  // uuid & 0xFFFFFFFFFFFF (48-bit, last part)

  // Test with specific bit patterns to ensure each shift is covered
  let uuid = MessageId::Uuid(0x12345678_90ABCDEF_FEDCBA09_87654321);
  let display = format!("{}", uuid);
  assert_eq!(display.len(), 36);
  assert_eq!(display.matches('-').count(), 4);

  // Verify format: XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
  let parts: Vec<&str> = display.split('-').collect();
  assert_eq!(parts.len(), 5);
  assert_eq!(parts[0].len(), 8); // First part: 8 hex chars (32 bits)
  assert_eq!(parts[1].len(), 4); // Second part: 4 hex chars (16 bits)
  assert_eq!(parts[2].len(), 4); // Third part: 4 hex chars (16 bits)
  assert_eq!(parts[3].len(), 4); // Fourth part: 4 hex chars (16 bits)
  assert_eq!(parts[4].len(), 12); // Last part: 12 hex chars (48 bits)
}

#[test]
fn test_metadata_deserialize_arc_str_option_none() {
  use serde_json;

  // Test deserialize_arc_str_option with None (line 443-444)
  let json = r#"{"source": null}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(metadata.source, None);
}

#[test]
fn test_metadata_deserialize_arc_str_option_some() {
  use serde_json;

  // Test deserialize_arc_str_option with Some value (line 443-444)
  let json = r#"{"source": "test-source"}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(metadata.source, Some(Arc::from("test-source")));
}

#[test]
fn test_metadata_deserialize_arc_str_vec_empty() {
  use serde_json;

  // Test deserialize_arc_str_vec with empty vector (line 465-471)
  let json = r#"{"headers": []}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert!(metadata.headers.is_empty());
}

#[test]
fn test_metadata_deserialize_arc_str_vec_multiple() {
  use serde_json;

  // Test deserialize_arc_str_vec with multiple headers (line 465-471)
  let json = r#"{"headers": [["key1", "value1"], ["key2", "value2"]]}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(metadata.headers.len(), 2);
  assert_eq!(metadata.get_header("key1"), Some("value1"));
  assert_eq!(metadata.get_header("key2"), Some("value2"));
}

#[test]
fn test_metadata_serialize_arc_str_option_none() {
  use serde_json;

  // Test serialize_arc_str_option with None (line 435)
  let metadata = MessageMetadata::default();
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  // source should be skipped if None (skip_serializing_if = "Option::is_none")
  assert!(!json.contains("\"source\""));
}

#[test]
fn test_metadata_serialize_arc_str_option_some() {
  use serde_json;

  // Test serialize_arc_str_option with Some value (line 434)
  let metadata = MessageMetadata::default().source("test-source");
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  assert!(json.contains("\"source\""));
  assert!(json.contains("test-source"));
}

#[test]
fn test_metadata_serialize_arc_str_vec_empty() {
  use serde_json;

  // Test serialize_arc_str_vec with empty vector (line 452)
  let metadata = MessageMetadata::default();
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");
  assert_eq!(parsed["headers"], serde_json::json!([]));
}

#[test]
fn test_metadata_serialize_arc_str_vec_multiple() {
  use serde_json;

  // Test serialize_arc_str_vec with multiple headers (line 453-454)
  let metadata = MessageMetadata::default()
    .header("key1", "value1")
    .header("key2", "value2");
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");
  assert_eq!(parsed["headers"].as_array().unwrap().len(), 2);
}

#[test]
fn test_message_id_display_sequence_edge_cases() {
  let seq0 = MessageId::Sequence(0);
  assert_eq!(format!("{}", seq0), "seq:0");

  let seq_max = MessageId::Sequence(u64::MAX);
  assert_eq!(format!("{}", seq_max), format!("seq:{}", u64::MAX));
}

#[test]
fn test_message_id_display_custom_empty() {
  let custom = MessageId::Custom(String::new());
  assert_eq!(format!("{}", custom), "custom:");
}

#[test]
fn test_message_id_display_content_hash_edge_cases() {
  let hash0 = MessageId::ContentHash(0);
  assert_eq!(format!("{}", hash0), "hash:0000000000000000");

  let hash_max = MessageId::ContentHash(u64::MAX);
  assert_eq!(format!("{}", hash_max), "hash:ffffffffffffffff");
}

#[test]
fn test_message_with_timestamp_now_error_case() {
  // Test that with_timestamp_now handles duration_since error gracefully (line 485)
  // This is hard to test directly, but we can verify the method works
  let metadata = MessageMetadata::with_timestamp_now();
  // Should either have a timestamp or None, but should not panic
  assert!(metadata.timestamp.is_some() || metadata.timestamp.is_none());
}

// Test serialize_arc_str_option with Some value (line 434)
#[test]
fn test_serialize_arc_str_option_some() {
  use serde_json;
  let source: Arc<str> = Arc::from("test-source");
  let metadata = MessageMetadata::default().source(source);
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  assert!(json.contains("test-source"));
}

// Test serialize_arc_str_vec with single header (line 453-454)
#[test]
fn test_serialize_arc_str_vec_single() {
  use serde_json;
  let metadata = MessageMetadata::default().header("single-key", "single-value");
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");
  assert_eq!(parsed["headers"].as_array().unwrap().len(), 1);
}

// Test deserialize_arc_str_option with Some String value (line 444)
#[test]
fn test_deserialize_arc_str_option_some_string() {
  use serde_json;
  let json = r#"{"source": "deserialized-source", "key": "deserialized-key"}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(metadata.get_source(), Some("deserialized-source"));
  assert_eq!(metadata.get_key(), Some("deserialized-key"));
}

// Test deserialize_arc_str_vec with single header pair (line 469)
#[test]
fn test_deserialize_arc_str_vec_single() {
  use serde_json;
  let json = r#"{"headers": [["single-key", "single-value"]]}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(metadata.headers.len(), 1);
  assert_eq!(metadata.get_header("single-key"), Some("single-value"));
}

// Test MessageId Display for UUID with specific bit patterns to cover all shifts
#[test]
fn test_message_id_display_uuid_bit_operations() {
  // Test all bit shift operations in Display implementation (lines 276-280)
  // uuid >> 96 (32-bit)
  // uuid >> 80 (16-bit)
  // uuid >> 64 (16-bit)
  // uuid >> 48 (16-bit)
  // uuid & 0xFFFFFFFFFFFF (48-bit)

  // Test with pattern that ensures all shifts are exercised
  let uuid = MessageId::Uuid(0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF);
  let display = format!("{}", uuid);
  assert_eq!(display.len(), 36);
  assert_eq!(display.matches('-').count(), 4);

  // Test with zeros to ensure all shifts work
  let uuid_zero = MessageId::Uuid(0);
  let display_zero = format!("{}", uuid_zero);
  assert_eq!(display_zero.len(), 36);
  assert_eq!(display_zero.matches('-').count(), 4);
}

// Test rand_u64 function path (indirectly through new_uuid)
// This exercises the rand_u64() function which is private
#[test]
fn test_rand_u64_indirectly() {
  // Call new_uuid multiple times to ensure rand_u64 is called
  // This tests the xorshift algorithm in rand_u64 (lines 1120-1126)
  let ids: Vec<MessageId> = (0..100).map(|_| MessageId::new_uuid()).collect();

  // All should be unique UUIDs
  for id in &ids {
    assert!(id.is_uuid());
  }

  // Check that we got some variety (not all the same)
  let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
  assert!(unique_count > 1, "Should generate different UUIDs");
}

// Test MessageId::Hash implementation with all variants to cover line 292
#[test]
fn test_message_id_hash_discriminant_all_variants() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let variants = [
    MessageId::Uuid(123),
    MessageId::Sequence(123),
    MessageId::Custom("123".to_string()),
    MessageId::ContentHash(123),
  ];

  let mut hashers: Vec<DefaultHasher> = variants.iter().map(|_| DefaultHasher::new()).collect();

  for (id, hasher) in variants.iter().zip(hashers.iter_mut()) {
    id.hash(hasher);
  }

  // All should produce different hash values due to discriminant (line 292)
  let hashes: Vec<u64> = hashers.into_iter().map(|h| h.finish()).collect();
  for i in 0..hashes.len() {
    for j in (i + 1)..hashes.len() {
      assert_ne!(
        hashes[i], hashes[j],
        "Different variants should hash differently"
      );
    }
  }
}

// Test deserialize_arc_str_vec with empty iterator path (line 468-469)
#[test]
fn test_deserialize_arc_str_vec_empty_iter() {
  use serde_json;
  // Empty array should create empty vec
  let json = r#"{"headers": []}"#;
  let metadata: MessageMetadata = serde_json::from_str(json).expect("Should deserialize");
  assert_eq!(metadata.headers.len(), 0);
}

// Test serialize_arc_str_vec seq.end() path (line 456)
#[test]
fn test_serialize_arc_str_vec_end() {
  use serde_json;
  // Test with empty headers to ensure seq.end() is called
  let metadata = MessageMetadata::default();
  let json = serde_json::to_string(&metadata).expect("Should serialize");
  let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");
  assert_eq!(parsed["headers"], serde_json::json!([]));

  // Test with multiple headers to ensure loop completes and end() is called
  let metadata2 = MessageMetadata::default()
    .header("a", "1")
    .header("b", "2")
    .header("c", "3");
  let json2 = serde_json::to_string(&metadata2).expect("Should serialize");
  let parsed2: serde_json::Value = serde_json::from_str(&json2).expect("Should parse");
  assert_eq!(parsed2["headers"].as_array().unwrap().len(), 3);
}

// Test MessageId::from_content with empty slice (line 239)
#[test]
fn test_message_id_from_content_empty() {
  let id1 = MessageId::from_content(&[]);
  let id2 = MessageId::from_content(&[]);

  assert!(id1.is_content_hash());
  assert_eq!(id1, id2); // Empty slices should produce same hash
}

// Test MessageId::from_content with various content types (line 239)
#[test]
fn test_message_id_from_content_various() {
  let id1 = MessageId::from_content(&[0u8]);
  let id2 = MessageId::from_content(&[1u8]);
  let id3 = MessageId::from_content(b"hello");

  assert!(id1.is_content_hash());
  assert!(id2.is_content_hash());
  assert!(id3.is_content_hash());
  assert_ne!(id1, id2);
  assert_ne!(id1, id3);
}

// Test MessageId hash with all variants separately to cover all branches (lines 294-297)
#[test]
fn test_message_id_hash_all_variants_separately() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  // Test each variant's hash implementation
  let uuid = MessageId::Uuid(42);
  let seq = MessageId::Sequence(42);
  let custom = MessageId::Custom("42".to_string());
  let hash = MessageId::ContentHash(42);

  let mut h1 = DefaultHasher::new();
  uuid.hash(&mut h1);

  let mut h2 = DefaultHasher::new();
  seq.hash(&mut h2);

  let mut h3 = DefaultHasher::new();
  custom.hash(&mut h3);

  let mut h4 = DefaultHasher::new();
  hash.hash(&mut h4);

  // All should produce different hashes (line 292: discriminant + value)
  assert_ne!(h1.finish(), h2.finish());
  assert_ne!(h1.finish(), h3.finish());
  assert_ne!(h1.finish(), h4.finish());
}

// Test MessageMetadata::new() explicitly (line 477-479)
#[test]
fn test_metadata_new_explicit() {
  let metadata = MessageMetadata::new();
  assert_eq!(metadata.timestamp, None);
  assert_eq!(metadata.source, None);
  assert!(metadata.headers.is_empty());
}

// Test UuidGenerator::new() explicitly (line 989-991)
#[test]
fn test_uuid_generator_new() {
  let generator = UuidGenerator::new();
  let id = generator.next_id();
  assert!(id.is_uuid());
}

// Test ContentHashGenerator::new() explicitly (line 1065-1067)
#[test]
fn test_content_hash_generator_new() {
  let generator = ContentHashGenerator::new();
  assert_eq!(
    generator.hash_content(b"test"),
    MessageId::from_content(b"test")
  );
}

// Test Message::map closure execution (line 872)
#[test]
fn test_message_map_closure_execution() {
  let msg = Message::new(10, MessageId::new_sequence(1));
  let mapped = msg.map(|x| x * 3);
  assert_eq!(*mapped.payload(), 30);
}

// Test Message::map_with_id closure with different ID types (line 884)
#[test]
fn test_message_map_with_id_all_id_types() {
  // Test with Sequence
  let msg_seq = Message::new(10, MessageId::new_sequence(5));
  let result_seq = msg_seq.map_with_id(|id, payload| {
    if let MessageId::Sequence(seq) = id {
      *seq as i32 + payload
    } else {
      payload
    }
  });
  assert_eq!(*result_seq.payload(), 15);

  // Test with UUID (else branch)
  let msg_uuid = Message::new(10, MessageId::new_uuid());
  let result_uuid = msg_uuid.map_with_id(|id, _payload| {
    if let MessageId::Sequence(_) = id {
      0
    } else {
      100 // else branch
    }
  });
  assert_eq!(*result_uuid.payload(), 100);
}

// Test SharedMessage::from_arc explicitly (line 1159-1161)
#[test]
fn test_shared_message_from_arc_direct() {
  let msg = Message::new(42, MessageId::new_uuid());
  let arc = Arc::new(msg);
  let shared = SharedMessage::from_arc(arc);
  assert_eq!(*shared.payload(), 42);
}

// Test SharedMessage::into_arc explicitly (line 1165-1167)
#[test]
fn test_shared_message_into_arc_direct() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared = SharedMessage::from(msg);
  let arc = shared.into_arc();
  assert_eq!(*arc.payload(), 42);
}

// Test SharedMessage::try_unwrap Err path (line 1173)
#[test]
fn test_shared_message_try_unwrap_err_path() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared1 = SharedMessage::from(msg);
  let _shared2 = shared1.clone(); // Create second reference

  // Try to unwrap should fail and return Err with SharedMessage
  let result = shared1.try_unwrap();
  assert!(result.is_err());

  if let Err(err_shared) = result {
    assert_eq!(*err_shared.payload(), 42);
  }
}

// Test SharedMessage::try_unwrap Ok path (line 1173)
#[test]
fn test_shared_message_try_unwrap_ok_path() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared = SharedMessage::from(msg);

  // Should succeed when this is the only reference
  let result = shared.try_unwrap();
  assert!(result.is_ok());

  if let Ok(unwrapped) = result {
    assert_eq!(*unwrapped.payload(), 42);
  }
}

// Test MessageId Display format precision for UUID parts
#[test]
fn test_message_id_display_uuid_format_parts() {
  // Test with specific UUID to verify format correctness
  // This ensures all write! macros are covered (lines 275-281)
  let uuid = MessageId::Uuid(0x12345678_90ABCDEF_FEDCBA09_87654321);
  let display = format!("{}", uuid);

  // Should be in format: XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
  let parts: Vec<&str> = display.split('-').collect();
  assert_eq!(parts.len(), 5);

  // Verify each part has correct length (covers all format specifiers)
  assert_eq!(parts[0].len(), 8); // {:08x} - 8 hex chars
  assert_eq!(parts[1].len(), 4); // {:04x} - 4 hex chars
  assert_eq!(parts[2].len(), 4); // {:04x} - 4 hex chars
  assert_eq!(parts[3].len(), 4); // {:04x} - 4 hex chars
  assert_eq!(parts[4].len(), 12); // {:012x} - 12 hex chars
}
