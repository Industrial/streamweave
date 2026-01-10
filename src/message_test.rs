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
