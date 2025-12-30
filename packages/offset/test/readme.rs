//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use streamweave_offset::{
  CommitStrategy, InMemoryOffsetStore, Offset, OffsetResetPolicy, OffsetTracker,
};
use tempfile::tempdir;

#[test]
fn test_basic_offset_tracking() {
  // Example from README.md lines 34-50
  // Create an in-memory offset store
  let store = Box::new(InMemoryOffsetStore::new());

  // Create an offset tracker with auto-commit
  let tracker = OffsetTracker::new(store);

  // Record processed offsets
  tracker.record("my-source", Offset::Sequence(100)).unwrap();
  tracker.record("my-source", Offset::Sequence(101)).unwrap();

  // Get the current offset
  let current = tracker.get_offset("my-source").unwrap();
  assert_eq!(current, Offset::Sequence(101));
}

#[test]
fn test_file_based_persistence() {
  // Example from README.md lines 52-70
  let dir = tempdir().unwrap();
  let path = dir.path().join("offsets.json");

  // Create a file-based offset store
  let store = Box::new(streamweave_offset::FileOffsetStore::new(&path).unwrap());

  // Create tracker
  let tracker = OffsetTracker::new(store);

  // Record offsets (automatically persisted)
  tracker
    .record("kafka-topic", Offset::Sequence(1000))
    .unwrap();

  // After restart, offsets are automatically loaded
  let store2 = Box::new(streamweave_offset::FileOffsetStore::new(&path).unwrap());
  let tracker2 = OffsetTracker::new(store2);
  let offset = tracker2.get_offset("kafka-topic").unwrap();
  // offset is Sequence(1000)
  assert_eq!(offset, Offset::Sequence(1000));
}

#[test]
fn test_auto_commit_strategy() {
  // Example from README.md lines 159-176
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);

  // Each record is immediately committed
  tracker.record("source", Offset::Sequence(1)).unwrap();
  tracker.record("source", Offset::Sequence(2)).unwrap();

  // Offset is immediately available
  let offset = tracker.get_offset("source").unwrap();
  assert_eq!(offset, Offset::Sequence(2));
}

#[test]
fn test_periodic_commit_strategy() {
  // Example from README.md lines 178-201
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(
    store,
    CommitStrategy::Periodic(10), // Commit every 10 items
  );

  // Record 9 items (not yet committed)
  for i in 1..=9 {
    tracker.record("source", Offset::Sequence(i)).unwrap();
  }
  assert!(tracker.get_all_committed().unwrap().is_empty());

  // 10th item triggers commit
  tracker.record("source", Offset::Sequence(10)).unwrap();
  let offset = tracker.get_offset("source").unwrap();
  assert_eq!(offset, Offset::Sequence(10));
}

#[test]
fn test_manual_commit_strategy() {
  // Example from README.md lines 203-230
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  // Record offsets (not committed)
  tracker.record("source", Offset::Sequence(100)).unwrap();
  tracker.record("source", Offset::Sequence(200)).unwrap();

  // Check pending offsets
  let pending = tracker.get_all_pending().unwrap();
  assert_eq!(pending.get("source"), Some(&Offset::Sequence(200)));

  // Manually commit
  tracker.commit("source").unwrap();

  // Now committed
  let offset = tracker.get_offset("source").unwrap();
  assert_eq!(offset, Offset::Sequence(200));
}

#[test]
fn test_offset_reset_policies() {
  // Example from README.md lines 232-262
  // Earliest policy (default) - start from beginning
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Earliest);

  let offset = tracker.get_offset("new-source").unwrap();
  assert_eq!(offset, Offset::Earliest);

  // Latest policy - start from latest
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Latest);

  let offset = tracker.get_offset("new-source").unwrap();
  assert_eq!(offset, Offset::Latest);

  // None policy - fail if no offset found
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::None);

  let result = tracker.get_offset("new-source");
  assert!(result.is_err());
}

#[test]
fn test_file_based_persistence_extended() {
  // Example from README.md lines 264-288
  let dir = tempdir().unwrap();
  let path = dir.path().join("offsets.json");

  // Create file-based store
  let store = Box::new(streamweave_offset::FileOffsetStore::new(&path).unwrap());
  let tracker = OffsetTracker::new(store);

  // Record offsets (automatically persisted to disk)
  tracker.record("topic-1", Offset::Sequence(100)).unwrap();
  tracker.record("topic-2", Offset::Sequence(200)).unwrap();

  // After restart, create new tracker with same file
  let store2 = Box::new(streamweave_offset::FileOffsetStore::new(&path).unwrap());
  let tracker2 = OffsetTracker::new(store2);

  // Offsets are automatically loaded
  let offset1 = tracker2.get_offset("topic-1").unwrap();
  let offset2 = tracker2.get_offset("topic-2").unwrap();
  assert_eq!(offset1, Offset::Sequence(100));
  assert_eq!(offset2, Offset::Sequence(200));
}

#[test]
fn test_multiple_sources() {
  // Example from README.md lines 352-373
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);

  // Track offsets for multiple sources
  tracker
    .record("kafka-topic-1", Offset::Sequence(100))
    .unwrap();
  tracker
    .record("kafka-topic-2", Offset::Sequence(200))
    .unwrap();
  tracker
    .record("file-source", Offset::Custom("line-5000".to_string()))
    .unwrap();

  // Get all committed offsets
  let all_offsets = tracker.get_all_committed().unwrap();
  assert_eq!(all_offsets.len(), 3);

  // Commit all pending offsets
  tracker.commit_all().unwrap();
}
