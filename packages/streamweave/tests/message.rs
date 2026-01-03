//! Tests for Message module functionality

use futures::StreamExt;
use std::sync::Arc;
use streamweave::message::{
  IdGenerator, Message, MessageId, MessageMetadata, MessageStreamExt, SequenceGenerator,
  SharedMessage, UuidGenerator, unwrap_message, wrap_message, wrap_message_with_generator,
  wrap_message_with_id, wrap_message_with_metadata, wrap_messages,
};

#[test]
fn test_message_creation() {
  let msg = Message::new(42, MessageId::new_uuid());
  assert_eq!(*msg.payload(), 42);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_message_with_metadata() {
  let id = MessageId::new_sequence(1);
  let metadata = MessageMetadata::with_timestamp_now().source("test-source");
  let msg = Message::with_metadata(42, id, metadata);

  assert_eq!(*msg.payload(), 42);
  assert!(msg.id().is_sequence());
  assert_eq!(msg.metadata().get_source(), Some("test-source"));
}

#[test]
fn test_message_id_types() {
  // UUID
  let uuid_id = MessageId::new_uuid();
  assert!(uuid_id.is_uuid());

  // Sequence
  let seq_id = MessageId::new_sequence(42);
  assert!(seq_id.is_sequence());

  // Custom
  let custom_id = MessageId::new_custom("my-id");
  assert!(custom_id.is_custom());

  // Content hash
  let hash_id = MessageId::from_content(b"test content");
  assert!(hash_id.is_content_hash());
}

#[test]
fn test_message_id_display() {
  let seq_id = MessageId::new_sequence(42);
  let display = format!("{}", seq_id);
  assert!(display.starts_with("seq:"));

  let custom_id = MessageId::new_custom("test-id");
  let display = format!("{}", custom_id);
  assert!(display.starts_with("custom:"));
}

#[test]
fn test_message_metadata() {
  let mut metadata = MessageMetadata::new();
  metadata = metadata
    .source("my-source")
    .partition(0)
    .offset(100)
    .key("my-key");

  assert_eq!(metadata.get_source(), Some("my-source"));
  assert_eq!(metadata.partition, Some(0));
  assert_eq!(metadata.offset, Some(100));
  assert_eq!(metadata.get_key(), Some("my-key"));
}

#[test]
fn test_message_metadata_headers() {
  let metadata = MessageMetadata::new()
    .header("Content-Type", "application/json")
    .header("X-Custom", "value");

  assert_eq!(
    metadata.get_header("Content-Type"),
    Some("application/json")
  );
  assert_eq!(metadata.get_header("X-Custom"), Some("value"));
  assert_eq!(metadata.get_header("NonExistent"), None);
}

#[test]
fn test_message_payload_access() {
  let msg = Message::new(42, MessageId::new_uuid());
  assert_eq!(*msg.payload(), 42);

  let mut msg = msg;
  *msg.payload_mut() = 100;
  assert_eq!(*msg.payload(), 100);
}

#[test]
fn test_message_map() {
  let msg = Message::new(42, MessageId::new_uuid());
  let id = msg.id().clone();

  let doubled = msg.map(|x| x * 2);
  assert_eq!(*doubled.payload(), 84);
  assert_eq!(*doubled.id(), id); // ID preserved
}

#[test]
fn test_message_map_with_id() {
  let msg = Message::new(42, MessageId::new_sequence(1));

  let transformed = msg.map_with_id(|id, payload| {
    if id.is_sequence() {
      payload * 2
    } else {
      payload
    }
  });

  assert_eq!(*transformed.payload(), 84);
}

#[test]
fn test_message_with_payload() {
  let msg = Message::new(42, MessageId::new_uuid());
  let id = msg.id().clone();
  let metadata = msg.metadata().clone();

  let new_msg = msg.with_payload("hello");
  assert_eq!(*new_msg.payload(), "hello");
  assert_eq!(*new_msg.id(), id); // ID preserved
  // Metadata preserved (check source as a proxy for equality)
  assert_eq!(new_msg.metadata().get_source(), metadata.get_source());
}

#[test]
fn test_message_into_parts() {
  let id = MessageId::new_sequence(1);
  let metadata = MessageMetadata::new().source("test");
  let msg = Message::with_metadata(42, id.clone(), metadata.clone());

  let (retrieved_id, payload, retrieved_metadata) = msg.into_parts();
  assert_eq!(retrieved_id, id);
  assert_eq!(payload, 42);
  assert_eq!(retrieved_metadata.get_source(), metadata.get_source());
}

#[test]
fn test_message_into_payload() {
  let msg = Message::new(42, MessageId::new_uuid());
  let payload = msg.into_payload();
  assert_eq!(payload, 42);
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

  if let (MessageId::Sequence(s1), MessageId::Sequence(s2), MessageId::Sequence(s3)) =
    (id1, id2, id3)
  {
    assert_eq!(s1, 0);
    assert_eq!(s2, 1);
    assert_eq!(s3, 2);
  } else {
    panic!("Expected sequence IDs");
  }
}

#[test]
fn test_sequence_generator_starting_at() {
  let generator = SequenceGenerator::starting_at(100);
  let id = generator.next_id();

  if let MessageId::Sequence(seq) = id {
    assert_eq!(seq, 100);
  } else {
    panic!("Expected sequence ID");
  }
}

#[test]
fn test_shared_message() {
  let msg = Message::new(42, MessageId::new_uuid());
  let shared1 = SharedMessage::from(msg);

  assert_eq!(*shared1.payload(), 42);

  let shared2 = shared1.clone();
  assert_eq!(*shared2.payload(), 42);

  // Test that they share the same Arc
  let arc1 = shared1.into_arc();
  let arc2 = shared2.into_arc();
  assert_eq!(Arc::strong_count(&arc1), 2); // Both arcs point to same data
  drop(arc2);
  assert_eq!(Arc::strong_count(&arc1), 1);
}

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
  assert_eq!(*msg.id(), id);
}

#[test]
fn test_wrap_message_with_metadata() {
  let id = MessageId::new_uuid();
  let metadata = MessageMetadata::new().source("test");
  let msg = wrap_message_with_metadata(42, id.clone(), metadata.clone());

  assert_eq!(*msg.payload(), 42);
  assert_eq!(*msg.id(), id);
  assert_eq!(msg.metadata().get_source(), metadata.get_source());
}

#[test]
fn test_unwrap_message() {
  let msg = Message::new(42, MessageId::new_uuid());
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

  // Check that IDs are sequential
  if let (MessageId::Sequence(s1), MessageId::Sequence(s2), MessageId::Sequence(s3)) = (
    messages[0].id().clone(),
    messages[1].id().clone(),
    messages[2].id().clone(),
  ) {
    assert_eq!(s1, 0);
    assert_eq!(s2, 1);
    assert_eq!(s3, 2);
  } else {
    panic!("Expected sequence IDs");
  }
}

#[tokio::test]
async fn test_message_stream_extract_payloads() {
  use futures::stream;

  let messages = stream::iter(vec![
    Message::new(1, MessageId::new_uuid()),
    Message::new(2, MessageId::new_uuid()),
    Message::new(3, MessageId::new_uuid()),
  ]);

  let payloads: Vec<i32> = messages.extract_payloads().collect().await;
  assert_eq!(payloads, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_message_stream_extract_ids() {
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
  assert_eq!(ids, vec![id1, id2, id3]);
}

#[tokio::test]
async fn test_message_stream_extract_metadata() {
  use futures::stream;

  let metadata1 = MessageMetadata::new().source("source1");
  let metadata2 = MessageMetadata::new().source("source2");

  let messages = stream::iter(vec![
    Message::with_metadata(1, MessageId::new_uuid(), metadata1.clone()),
    Message::with_metadata(2, MessageId::new_uuid(), metadata2.clone()),
  ]);

  let metadatas: Vec<MessageMetadata> = messages.extract_metadata().collect().await;
  assert_eq!(metadatas.len(), 2);
  assert_eq!(metadatas[0].get_source(), metadata1.get_source());
  assert_eq!(metadatas[1].get_source(), metadata2.get_source());
}

#[tokio::test]
async fn test_message_stream_map_payload() {
  use futures::stream;

  let id = MessageId::new_sequence(1);
  let messages = stream::iter(vec![
    Message::new(1, id.clone()),
    Message::new(2, id.clone()),
  ]);

  let doubled: Vec<Message<i32>> = messages.map_payload(|x| x * 2).collect().await;
  assert_eq!(doubled.len(), 2);
  assert_eq!(*doubled[0].payload(), 2);
  assert_eq!(*doubled[1].payload(), 4);
  // IDs should be preserved
  assert_eq!(*doubled[0].id(), id);
}

#[test]
fn test_message_default() {
  let msg: Message<i32> = Message::default();
  assert_eq!(*msg.payload(), 0);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_message_hash() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let id = MessageId::new_sequence(1);
  let msg1 = Message::new(42, id.clone());
  let msg2 = Message::new(42, id.clone());

  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  msg1.hash(&mut hasher1);
  msg2.hash(&mut hasher2);

  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_message_metadata_builder_pattern() {
  let metadata = MessageMetadata::new()
    .timestamp(std::time::Duration::from_secs(1000))
    .source("my-source")
    .partition(0)
    .offset(100)
    .key("my-key")
    .header("Header1", "Value1")
    .header("Header2", "Value2");

  assert_eq!(
    metadata.timestamp,
    Some(std::time::Duration::from_secs(1000))
  );
  assert_eq!(metadata.get_source(), Some("my-source"));
  assert_eq!(metadata.partition, Some(0));
  assert_eq!(metadata.offset, Some(100));
  assert_eq!(metadata.get_key(), Some("my-key"));
  assert_eq!(metadata.get_header("Header1"), Some("Value1"));
  assert_eq!(metadata.get_header("Header2"), Some("Value2"));
}

#[test]
fn test_message_metadata_arc_sharing() {
  use std::sync::Arc;

  let source: Arc<str> = Arc::from("shared-source");
  let metadata1 = MessageMetadata::new().with_shared_source(source.clone());
  let metadata2 = MessageMetadata::new().with_shared_source(source.clone());

  // Both should share the same Arc
  assert!(Arc::ptr_eq(
    metadata1.source.as_ref().unwrap(),
    metadata2.source.as_ref().unwrap()
  ));
}
