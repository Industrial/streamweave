//! Tests for offset module

use chrono::Utc;
use std::collections::HashMap;
use streamweave::offset::*;

#[test]
fn test_offset_sequence() {
  let offset = Offset::sequence(42);
  match offset {
    Offset::Sequence(n) => assert_eq!(n, 42),
    _ => assert!(false),
  }
}

#[test]
fn test_offset_timestamp() {
  let now = Utc::now();
  let offset = Offset::timestamp(now);
  match offset {
    Offset::Timestamp(ts) => assert_eq!(ts, now),
    _ => assert!(false),
  }
}

#[test]
fn test_offset_custom() {
  let offset = Offset::custom("my-offset");
  match offset {
    Offset::Custom(s) => assert_eq!(s, "my-offset"),
    _ => assert!(false),
  }
}

#[test]
fn test_offset_earliest() {
  let offset = Offset::Earliest;
  assert!(offset.is_earliest());
  assert!(!offset.is_latest());
}

#[test]
fn test_offset_latest() {
  let offset = Offset::Latest;
  assert!(offset.is_latest());
  assert!(!offset.is_earliest());
}

#[test]
fn test_offset_increment() {
  let offset = Offset::sequence(5);
  let incremented = offset.increment();

  assert!(incremented.is_some());
  match incremented.unwrap() {
    Offset::Sequence(n) => assert_eq!(n, 6),
    _ => assert!(false),
  }
}

#[test]
fn test_offset_increment_non_sequence() {
  let offset = Offset::Latest;
  assert!(offset.increment().is_none());
}

#[test]
fn test_offset_display() {
  let seq = Offset::sequence(42);
  assert!(format!("{}", seq).contains("seq:42"));

  let custom = Offset::custom("test");
  assert!(format!("{}", custom).contains("custom:test"));

  assert_eq!(format!("{}", Offset::Earliest), "earliest");
  assert_eq!(format!("{}", Offset::Latest), "latest");
}

#[test]
fn test_offset_reset_policy() {
  assert_eq!(OffsetResetPolicy::default(), OffsetResetPolicy::Earliest);
  assert_ne!(OffsetResetPolicy::Earliest, OffsetResetPolicy::Latest);
}

#[test]
fn test_commit_strategy() {
  assert_eq!(CommitStrategy::default(), CommitStrategy::Auto);
  assert_ne!(CommitStrategy::Auto, CommitStrategy::Manual);

  match CommitStrategy::Periodic(10) {
    CommitStrategy::Periodic(n) => assert_eq!(n, 10),
    _ => assert!(false),
  }
}

#[test]
fn test_offset_error_display() {
  let error = OffsetError::IoError(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "File not found",
  ));
  let display = format!("{}", error);
  assert!(display.contains("IO error"));
}

#[test]
fn test_offset_error_from_io_error() {
  let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test");
  let offset_error: OffsetError = io_error.into();

  match offset_error {
    OffsetError::IoError(_) => assert!(true),
    _ => assert!(false),
  }
}

#[test]
fn test_in_memory_offset_store_new() {
  let store = InMemoryOffsetStore::new();
  assert!(store.get("test").unwrap().is_none());
}

#[test]
fn test_in_memory_offset_store_with_offsets() {
  let mut offsets = HashMap::new();
  offsets.insert("source1".to_string(), Offset::sequence(100));
  offsets.insert("source2".to_string(), Offset::sequence(200));

  let store = InMemoryOffsetStore::with_offsets(offsets);

  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(100)));
  assert_eq!(store.get("source2").unwrap(), Some(Offset::sequence(200)));
}

#[test]
fn test_in_memory_offset_store_commit() {
  let store = InMemoryOffsetStore::new();

  store.commit("source1", Offset::sequence(100)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(100)));
}

#[test]
fn test_in_memory_offset_store_get_all() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(100)).unwrap();
  store.commit("source2", Offset::sequence(200)).unwrap();

  let all = store.get_all().unwrap();
  assert_eq!(all.len(), 2);
  assert_eq!(all.get("source1"), Some(&Offset::sequence(100)));
}

#[test]
fn test_in_memory_offset_store_clear() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(100)).unwrap();
  store.clear("source1").unwrap();

  assert!(store.get("source1").unwrap().is_none());
}

#[test]
fn test_in_memory_offset_store_clear_all() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(100)).unwrap();
  store.commit("source2", Offset::sequence(200)).unwrap();
  store.clear_all().unwrap();

  let all = store.get_all().unwrap();
  assert_eq!(all.len(), 0);
}

#[test]
fn test_file_offset_store_new() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("test_offsets.json");

  // Clean up if exists
  let _ = std::fs::remove_file(&path);

  let store = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store.path(), path);

  // Clean up
  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_file_offset_store_commit_and_get() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("test_offsets_commit.json");

  // Clean up if exists
  let _ = std::fs::remove_file(&path);

  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::sequence(100)).unwrap();

  // Create new store to test persistence
  let store2 = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store2.get("source1").unwrap(), Some(Offset::sequence(100)));

  // Clean up
  let _ = std::fs::remove_file(&path);
}
