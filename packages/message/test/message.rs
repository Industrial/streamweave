use streamweave_message::*;

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

#[test]
fn test_message_id_default() {
  let id = MessageId::default();
  assert!(id.is_uuid());
}

#[test]
fn test_message_id_hash() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let id1 = MessageId::new_sequence(42);
  let id2 = MessageId::new_sequence(42);
  let id3 = MessageId::new_sequence(43);

  let mut hasher1 = DefaultHasher::new();
  id1.hash(&mut hasher1);
  let hash1 = hasher1.finish();

  let mut hasher2 = DefaultHasher::new();
  id2.hash(&mut hasher2);
  let hash2 = hasher2.finish();

  let mut hasher3 = DefaultHasher::new();
  id3.hash(&mut hasher3);
  let hash3 = hasher3.finish();

  assert_eq!(hash1, hash2);
  assert_ne!(hash1, hash3);

  // Test different types hash differently
  let uuid_id = MessageId::new_uuid();
  let mut hasher_uuid = DefaultHasher::new();
  uuid_id.hash(&mut hasher_uuid);
  let hash_uuid = hasher_uuid.finish();
  assert_ne!(hash1, hash_uuid);
}

#[test]
fn test_message_id_display_uuid_edge_cases() {
  // Test UUID display with various values
  let uuid1 = MessageId::Uuid(0);
  let formatted1 = format!("{}", uuid1);
  assert!(formatted1.contains("-"));

  let uuid2 = MessageId::Uuid(u128::MAX);
  let formatted2 = format!("{}", uuid2);
  assert!(formatted2.contains("-"));
}

#[test]
fn test_message_metadata_default() {
  let metadata = MessageMetadata::default();
  assert_eq!(metadata.timestamp, None);
  assert_eq!(metadata.source, None);
  assert_eq!(metadata.partition, None);
  assert_eq!(metadata.offset, None);
  assert_eq!(metadata.key, None);
  assert!(metadata.headers.is_empty());
}

#[test]
fn test_message_metadata_with_timestamp_now() {
  let metadata = MessageMetadata::with_timestamp_now();
  assert!(metadata.timestamp.is_some());
}

#[test]
fn test_message_metadata_timestamp() {
  let duration = std::time::Duration::from_secs(1234567890);
  let metadata = MessageMetadata::new().timestamp(duration);
  assert_eq!(metadata.timestamp, Some(duration));
}

#[test]
fn test_message_metadata_source() {
  let metadata = MessageMetadata::new().source("my-source");
  assert_eq!(metadata.source, Some("my-source".to_string()));
}

#[test]
fn test_message_metadata_partition() {
  let metadata = MessageMetadata::new().partition(5);
  assert_eq!(metadata.partition, Some(5));
}

#[test]
fn test_message_metadata_offset() {
  let metadata = MessageMetadata::new().offset(100);
  assert_eq!(metadata.offset, Some(100));
}

#[test]
fn test_message_metadata_key() {
  let metadata = MessageMetadata::new().key("my-key");
  assert_eq!(metadata.key, Some("my-key".to_string()));
}

#[test]
fn test_message_metadata_multiple_headers() {
  let metadata = MessageMetadata::new()
    .header("header1", "value1")
    .header("header2", "value2")
    .header("header1", "value3"); // Duplicate key

  assert_eq!(metadata.headers.len(), 3);
  assert_eq!(metadata.get_header("header1"), Some("value1")); // First match
  assert_eq!(metadata.get_header("header2"), Some("value2"));
  assert_eq!(metadata.get_header("header3"), None);
}

#[test]
fn test_message_default() {
  let msg: Message<i32> = Message::default();
  assert_eq!(*msg.payload(), 0);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_message_payload_mut() {
  let mut msg = Message::new(42, MessageId::new_sequence(1));
  *msg.payload_mut() = 100;
  assert_eq!(*msg.payload(), 100);
}

#[test]
fn test_message_metadata_mut() {
  let mut msg = Message::new(42, MessageId::new_sequence(1));
  msg.metadata_mut().source = Some("new-source".to_string());
  assert_eq!(msg.metadata().source, Some("new-source".to_string()));
}

#[test]
fn test_message_into_payload() {
  let msg = Message::new("test".to_string(), MessageId::new_sequence(1));
  let payload = msg.into_payload();
  assert_eq!(payload, "test".to_string());
}

#[test]
fn test_message_map_with_id() {
  let msg = Message::new(42, MessageId::new_sequence(1));
  let mapped = msg.map_with_id(|id, x| {
    assert_eq!(*id, MessageId::new_sequence(1));
    x * 2
  });
  assert_eq!(*mapped.payload(), 84);
  assert_eq!(*mapped.id(), MessageId::new_sequence(1));
}

#[test]
fn test_message_with_payload() {
  let msg = Message::new(42, MessageId::new_sequence(1));
  let new_msg = msg.with_payload("hello".to_string());
  assert_eq!(*new_msg.payload(), "hello");
  assert_eq!(*new_msg.id(), MessageId::new_sequence(1));
}

#[test]
fn test_message_partial_eq() {
  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(2);

  let msg1 = Message::new(42, id1.clone());
  let msg2 = Message::new(42, id1.clone());
  let msg3 = Message::new(42, id2);
  let msg4 = Message::new(43, id1);

  assert_eq!(msg1, msg2);
  assert_ne!(msg1, msg3);
  assert_ne!(msg1, msg4);
}

#[test]
fn test_message_hash() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let id = MessageId::new_sequence(1);
  let msg1 = Message::new(42, id.clone());
  let msg2 = Message::new(42, id.clone());
  let msg3 = Message::new(43, id);

  let mut hasher1 = DefaultHasher::new();
  msg1.hash(&mut hasher1);
  let hash1 = hasher1.finish();

  let mut hasher2 = DefaultHasher::new();
  msg2.hash(&mut hasher2);
  let hash2 = hasher2.finish();

  let mut hasher3 = DefaultHasher::new();
  msg3.hash(&mut hasher3);
  let hash3 = hasher3.finish();

  assert_eq!(hash1, hash2);
  assert_ne!(hash1, hash3);
}

#[test]
fn test_message_eq() {
  let id = MessageId::new_sequence(1);
  let msg1 = Message::new(42, id.clone());
  let msg2 = Message::new(42, id);
  assert_eq!(msg1, msg2);
}

#[test]
fn test_content_hash_generator() {
  let generator = ContentHashGenerator::new();
  let id1 = generator.hash_content(b"hello");
  let id2 = generator.hash_content(b"hello");
  let id3 = generator.hash_content(b"world");

  assert!(id1.is_content_hash());
  assert_eq!(id1, id2);
  assert_ne!(id1, id3);
}

#[test]
fn test_content_hash_generator_default() {
  let generator = ContentHashGenerator::default();
  let id = generator.hash_content(b"test");
  assert!(id.is_content_hash());
}

#[test]
fn test_uuid_generator_default() {
  let generator = UuidGenerator::default();
  let id = generator.next_id();
  assert!(id.is_uuid());
}

#[test]
fn test_sequence_generator_default() {
  let generator = SequenceGenerator::default();
  let id = generator.next_id();
  assert_eq!(id, MessageId::Sequence(0));
}
