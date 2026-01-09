//! Tests for message module

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use streamweave::message::*;

#[test]
fn test_message_id_uuid() {
  let id1 = MessageId::new_uuid();
  let id2 = MessageId::new_uuid();

  assert!(id1.is_uuid());
  assert!(id2.is_uuid());
  assert_ne!(id1, id2); // UUIDs should be unique
}

#[test]
fn test_message_id_sequence() {
  let id = MessageId::new_sequence(42);
  assert!(id.is_sequence());
  assert!(!id.is_uuid());
  assert!(!id.is_custom());
  assert!(!id.is_content_hash());
}

#[test]
fn test_message_id_custom() {
  let id = MessageId::new_custom("my-id");
  assert!(id.is_custom());
  assert!(!id.is_uuid());
  assert!(!id.is_sequence());
}

#[test]
fn test_message_id_content_hash() {
  let id = MessageId::from_content(b"test content");
  assert!(id.is_content_hash());
}

#[test]
fn test_message_id_display() {
  let seq_id = MessageId::new_sequence(123);
  let display = format!("{}", seq_id);
  assert!(display.contains("seq:123"));

  let custom_id = MessageId::new_custom("my-id");
  let display = format!("{}", custom_id);
  assert!(display.contains("custom:my-id"));
}

#[test]
fn test_message_id_hash() {
  let id1 = MessageId::new_sequence(42);
  let id2 = MessageId::new_sequence(42);

  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  id1.hash(&mut hasher1);
  id2.hash(&mut hasher2);

  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_message_id_default() {
  let id = MessageId::default();
  assert!(id.is_uuid());
}

#[test]
fn test_message_metadata_default() {
  let metadata = MessageMetadata::default();
  assert_eq!(metadata.get_source(), None);
  assert_eq!(metadata.get_partition(), None);
  assert_eq!(metadata.get_offset(), None);
}

#[test]
fn test_message_metadata_with_timestamp() {
  let metadata = MessageMetadata::with_timestamp_now();
  assert!(metadata.get_timestamp().is_some());
}

#[test]
fn test_message_metadata_source() {
  let metadata = MessageMetadata::default().source("my-source");
  assert_eq!(metadata.get_source(), Some("my-source"));
}

#[test]
fn test_message_metadata_partition() {
  let metadata = MessageMetadata::default().partition(5);
  assert_eq!(metadata.get_partition(), Some(5));
}

#[test]
fn test_message_metadata_offset() {
  let metadata = MessageMetadata::default().offset(100);
  assert_eq!(metadata.get_offset(), Some(100));
}

#[test]
fn test_message_metadata_key() {
  let metadata = MessageMetadata::default().key("my-key");
  assert_eq!(metadata.get_key(), Some("my-key"));
}

#[test]
fn test_message_metadata_header() {
  let metadata = MessageMetadata::default()
    .header("key1", "value1")
    .header("key2", "value2");

  assert_eq!(metadata.get_header("key1"), Some("value1"));
  assert_eq!(metadata.get_header("key2"), Some("value2"));
  assert_eq!(metadata.get_header("key3"), None);
}

#[test]
fn test_message_metadata_headers() {
  let metadata = MessageMetadata::default().header("a", "1").header("b", "2");

  let headers = metadata.get_headers();
  assert_eq!(headers.len(), 2);
  assert_eq!(headers.get("a"), Some(&"1".into()));
}

#[test]
fn test_message_new() {
  let id = MessageId::new_uuid();
  let msg = Message::new(42, id.clone());

  assert_eq!(msg.payload(), &42);
  assert_eq!(msg.id(), &id);
}

#[test]
fn test_message_with_metadata() {
  let id = MessageId::new_uuid();
  let metadata = MessageMetadata::default().source("test");
  let msg = Message::with_metadata(42, id.clone(), metadata.clone());

  assert_eq!(msg.payload(), &42);
  assert_eq!(msg.id(), &id);
  assert_eq!(msg.metadata().get_source(), Some("test"));
}

#[test]
fn test_message_map() {
  let msg = wrap_message(42);
  let doubled = msg.map(|x| x * 2);

  assert_eq!(doubled.payload(), &84);
}

#[test]
fn test_message_into_payload() {
  let msg = wrap_message(42);
  let payload = msg.into_payload();

  assert_eq!(payload, 42);
}

#[test]
fn test_wrap_message() {
  let msg = wrap_message(42);
  assert_eq!(msg.payload(), &42);
  assert!(msg.id().is_uuid());
}

#[test]
fn test_wrap_message_with_id() {
  let id = MessageId::new_sequence(123);
  let msg = wrap_message_with_id(42, id.clone());

  assert_eq!(msg.payload(), &42);
  assert_eq!(msg.id(), &id);
}

#[test]
fn test_wrap_message_with_metadata() {
  let metadata = MessageMetadata::default().source("test");
  let msg = wrap_message_with_metadata(42, metadata.clone());

  assert_eq!(msg.payload(), &42);
  assert_eq!(msg.metadata().get_source(), Some("test"));
}

#[test]
fn test_wrap_messages() {
  let payloads = vec![1, 2, 3];
  let messages = wrap_messages(payloads);

  assert_eq!(messages.len(), 3);
  assert_eq!(messages[0].payload(), &1);
  assert_eq!(messages[1].payload(), &2);
  assert_eq!(messages[2].payload(), &3);
}

#[test]
fn test_unwrap_message() {
  let msg = wrap_message(42);
  let payload = unwrap_message(msg);

  assert_eq!(payload, 42);
}

#[tokio::test]
async fn test_message_stream_ext_extract_payloads() {
  use futures::StreamExt;
  use futures::stream;

  let messages = stream::iter(vec![wrap_message(1), wrap_message(2), wrap_message(3)]);

  let payloads: Vec<i32> = messages.extract_payloads().collect().await;
  assert_eq!(payloads, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_message_stream_ext_extract_ids() {
  use futures::StreamExt;
  use futures::stream;

  let id1 = MessageId::new_sequence(1);
  let id2 = MessageId::new_sequence(2);
  let messages = stream::iter(vec![
    wrap_message_with_id(1, id1.clone()),
    wrap_message_with_id(2, id2.clone()),
  ]);

  let ids: Vec<MessageId> = messages.extract_ids().collect().await;
  assert_eq!(ids.len(), 2);
}
