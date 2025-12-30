//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use streamweave_message::{Message, MessageId, MessageMetadata};

#[test]
fn test_creating_messages() {
  // Example from README.md lines 34-53
  // Create a simple message with UUID
  let msg = Message::new(42, MessageId::new_uuid());
  assert_eq!(*msg.payload(), 42);

  // Create a message with sequence ID
  let msg = Message::new("hello", MessageId::new_sequence(1));
  assert_eq!(*msg.payload(), "hello");

  // Create a message with metadata
  let metadata = MessageMetadata::with_timestamp_now()
    .source("my-source")
    .partition(0)
    .offset(100);

  let msg = Message::with_metadata("payload", MessageId::new_uuid(), metadata);
  assert_eq!(*msg.payload(), "payload");
  assert_eq!(msg.metadata().source, Some("my-source".to_string()));
}

#[test]
fn test_using_id_generators() {
  // Example from README.md lines 58-73
  use streamweave_message::{IdGenerator, SequenceGenerator, UuidGenerator};

  // UUID generator (globally unique)
  let uuid_gen = UuidGenerator::new();
  let id1 = uuid_gen.next_id();
  let id2 = uuid_gen.next_id();
  assert_ne!(id1, id2);

  // Sequence generator (monotonically increasing)
  let seq_gen = SequenceGenerator::new();
  let seq1 = seq_gen.next_id(); // Sequence(0)
  let seq2 = seq_gen.next_id(); // Sequence(1)
  assert_eq!(seq1, MessageId::Sequence(0));
  assert_eq!(seq2, MessageId::Sequence(1));

  // Sequence generator starting at specific value
  let seq_gen = SequenceGenerator::starting_at(1000);
  let seq = seq_gen.next_id(); // Sequence(1000)
  assert_eq!(seq, MessageId::Sequence(1000));
}

#[test]
fn test_creating_messages_with_different_id_types() {
  // Example from README.md lines 158-173
  // UUID-based message
  let msg = Message::new(42, MessageId::new_uuid());
  assert!(msg.id().is_uuid());

  // Sequence-based message
  let msg = Message::new("data", MessageId::new_sequence(1));
  assert!(msg.id().is_sequence());

  // Custom ID message
  let msg = Message::new(100, MessageId::new_custom("event-123"));
  assert!(msg.id().is_custom());

  // Content-hash based message
  let content = b"my content";
  let msg = Message::new(content, MessageId::from_content(content));
  assert!(msg.id().is_content_hash());
}

#[test]
fn test_working_with_metadata() {
  // Example from README.md lines 177-199
  // Create metadata with builder pattern
  let metadata = MessageMetadata::with_timestamp_now()
    .source("kafka-topic")
    .partition(3)
    .offset(1000)
    .key("user-123")
    .header("content-type", "application/json")
    .header("correlation-id", "req-456");

  let msg = Message::with_metadata("payload data", MessageId::new_uuid(), metadata);

  // Access metadata
  assert_eq!(msg.metadata().source, Some("kafka-topic".to_string()));
  assert_eq!(msg.metadata().partition, Some(3));
  assert_eq!(
    msg.metadata().get_header("content-type"),
    Some("application/json")
  );
}

#[test]
fn test_transforming_messages() {
  // Example from README.md lines 203-220
  let msg = Message::new(42, MessageId::new_sequence(1));

  // Map payload while preserving ID and metadata
  let doubled = msg.map(|x| x * 2);
  assert_eq!(*doubled.payload(), 84);
  assert_eq!(*doubled.id(), MessageId::new_sequence(1));

  // Map with access to message ID
  let msg = Message::new(42, MessageId::new_sequence(1));
  let with_id = msg.map_with_id(|id, payload| format!("{}:{}", id, payload));
  assert_eq!(*with_id.payload(), "seq:1:42");

  // Replace payload
  let msg = Message::new(42, MessageId::new_sequence(1));
  let new_msg = msg.with_payload("new payload");
  assert_eq!(*new_msg.payload(), "new payload");
}

#[test]
fn test_using_id_generators_extended() {
  // Example from README.md lines 224-249
  use streamweave_message::{IdGenerator, SequenceGenerator, UuidGenerator};

  // UUID generator
  let uuid_gen = UuidGenerator::new();
  for _ in 0..10 {
    let id = uuid_gen.next_id();
    assert!(id.is_uuid());
  }

  // Sequence generator
  let seq_gen = SequenceGenerator::new();
  let id1 = seq_gen.next_id(); // Sequence(0)
  let id2 = seq_gen.next_id(); // Sequence(1)
  assert_eq!(id1, MessageId::Sequence(0));
  assert_eq!(id2, MessageId::Sequence(1));

  // Sequence generator with starting value
  let seq_gen = SequenceGenerator::starting_at(100);
  let id = seq_gen.next_id(); // Sequence(100)
  assert_eq!(id, MessageId::Sequence(100));

  // Reset sequence
  let seq_gen = SequenceGenerator::new();
  seq_gen.reset();
  let id = seq_gen.next_id(); // Sequence(0)
  assert_eq!(id, MessageId::Sequence(0));

  // Get current value without incrementing
  let seq_gen = SequenceGenerator::new();
  let _ = seq_gen.next_id();
  let current = seq_gen.current();
  assert_eq!(current, 1);
}

#[test]
fn test_message_deduplication() {
  // Example from README.md lines 286-308
  use std::collections::HashSet;

  // Track seen message IDs
  let mut seen = HashSet::new();

  let messages = vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(1, MessageId::new_sequence(1)), // Duplicate
  ];

  let mut processed = 0;
  let mut skipped = 0;
  for msg in messages {
    if seen.insert(msg.id().clone()) {
      // Process unique message
      processed += 1;
    } else {
      // Skip duplicate
      skipped += 1;
    }
  }

  assert_eq!(processed, 2);
  assert_eq!(skipped, 1);
}

#[test]
fn test_message_routing_by_key() {
  // Example from README.md lines 312-344
  let messages = vec![
    Message::with_metadata(
      "data1",
      MessageId::new_uuid(),
      MessageMetadata::new().key("user-1"),
    ),
    Message::with_metadata(
      "data2",
      MessageId::new_uuid(),
      MessageMetadata::new().key("user-2"),
    ),
    Message::with_metadata(
      "data3",
      MessageId::new_uuid(),
      MessageMetadata::new().key("user-1"),
    ),
  ];

  // Route messages by key
  let mut user1_messages = vec![];
  let mut user2_messages = vec![];

  for msg in messages {
    match msg.metadata().key.as_deref() {
      Some("user-1") => user1_messages.push(msg),
      Some("user-2") => user2_messages.push(msg),
      _ => {}
    }
  }

  assert_eq!(user1_messages.len(), 2);
  assert_eq!(user2_messages.len(), 1);
}
