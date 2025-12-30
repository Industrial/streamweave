use std::collections::HashMap;
use streamweave_offset::*;
use tempfile::tempdir;

// Offset tests
#[test]
fn test_offset_display() {
  assert_eq!(Offset::Sequence(42).to_string(), "seq:42");
  assert_eq!(Offset::Earliest.to_string(), "earliest");
  assert_eq!(Offset::Latest.to_string(), "latest");
  assert_eq!(Offset::Custom("foo".to_string()).to_string(), "custom:foo");
}

#[test]
fn test_offset_increment() {
  assert_eq!(Offset::Sequence(0).increment(), Some(Offset::Sequence(1)));
  assert_eq!(Offset::Sequence(42).increment(), Some(Offset::Sequence(43)));
  assert_eq!(Offset::Earliest.increment(), None);
  assert_eq!(Offset::Latest.increment(), None);
  assert_eq!(Offset::Custom("x".to_string()).increment(), None);
}

#[test]
fn test_offset_is_earliest_latest() {
  assert!(Offset::Earliest.is_earliest());
  assert!(!Offset::Latest.is_earliest());
  assert!(Offset::Latest.is_latest());
  assert!(!Offset::Earliest.is_latest());
}

#[test]
fn test_offset_default() {
  assert_eq!(Offset::default(), Offset::Earliest);
}

// InMemoryOffsetStore tests
#[test]
fn test_in_memory_store_basic() {
  let store = InMemoryOffsetStore::new();

  assert!(store.get("source1").unwrap().is_none());

  store.commit("source1", Offset::Sequence(10)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::Sequence(10)));

  store.commit("source1", Offset::Sequence(20)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::Sequence(20)));
}

#[test]
fn test_in_memory_store_multiple_sources() {
  let store = InMemoryOffsetStore::new();

  store.commit("source1", Offset::Sequence(10)).unwrap();
  store.commit("source2", Offset::Sequence(20)).unwrap();

  assert_eq!(store.get("source1").unwrap(), Some(Offset::Sequence(10)));
  assert_eq!(store.get("source2").unwrap(), Some(Offset::Sequence(20)));

  let all = store.get_all().unwrap();
  assert_eq!(all.len(), 2);
}

#[test]
fn test_in_memory_store_clear() {
  let store = InMemoryOffsetStore::new();

  store.commit("source1", Offset::Sequence(10)).unwrap();
  store.commit("source2", Offset::Sequence(20)).unwrap();

  store.clear("source1").unwrap();
  assert!(store.get("source1").unwrap().is_none());
  assert_eq!(store.get("source2").unwrap(), Some(Offset::Sequence(20)));

  store.clear_all().unwrap();
  assert!(store.get_all().unwrap().is_empty());
}

#[test]
fn test_in_memory_store_with_initial_offsets() {
  let mut initial = HashMap::new();
  initial.insert("source1".to_string(), Offset::Sequence(100));

  let store = InMemoryOffsetStore::with_offsets(initial);
  assert_eq!(store.get("source1").unwrap(), Some(Offset::Sequence(100)));
}

// FileOffsetStore tests (native only)
#[test]
fn test_file_store_basic() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("offsets.json");

  let store = FileOffsetStore::new(&path).unwrap();

  assert!(store.get("source1").unwrap().is_none());

  store.commit("source1", Offset::Sequence(10)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::Sequence(10)));

  // Verify persistence
  let store2 = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store2.get("source1").unwrap(), Some(Offset::Sequence(10)));
}

#[test]
fn test_file_store_clear() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("offsets.json");

  let store = FileOffsetStore::new(&path).unwrap();

  store.commit("source1", Offset::Sequence(10)).unwrap();
  store.commit("source2", Offset::Sequence(20)).unwrap();

  store.clear("source1").unwrap();

  // Verify persistence of clear
  let store2 = FileOffsetStore::new(&path).unwrap();
  assert!(store2.get("source1").unwrap().is_none());
  assert_eq!(store2.get("source2").unwrap(), Some(Offset::Sequence(20)));
}

#[test]
fn test_file_store_creates_parent_dirs() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("nested/dir/offsets.json");

  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::Sequence(10)).unwrap();

  assert!(path.exists());
}

// OffsetTracker tests
#[test]
fn test_tracker_auto_commit() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);

  tracker.record("source1", Offset::Sequence(10)).unwrap();

  // With auto commit, offset should be immediately committed
  assert_eq!(
    tracker.get_all_committed().unwrap().get("source1"),
    Some(&Offset::Sequence(10))
  );
}

#[test]
fn test_tracker_periodic_commit() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(3));

  tracker.record("source1", Offset::Sequence(1)).unwrap();
  tracker.record("source1", Offset::Sequence(2)).unwrap();

  // Not committed yet
  assert!(tracker.get_all_committed().unwrap().is_empty());

  tracker.record("source1", Offset::Sequence(3)).unwrap();

  // Now committed (after 3rd record)
  assert_eq!(
    tracker.get_all_committed().unwrap().get("source1"),
    Some(&Offset::Sequence(3))
  );
}

#[test]
fn test_tracker_manual_commit() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  tracker.record("source1", Offset::Sequence(10)).unwrap();

  // Not committed yet
  assert!(tracker.get_all_committed().unwrap().is_empty());

  // Pending should have the offset
  assert_eq!(
    tracker.get_all_pending().unwrap().get("source1"),
    Some(&Offset::Sequence(10))
  );

  tracker.commit("source1").unwrap();

  // Now committed
  assert_eq!(
    tracker.get_all_committed().unwrap().get("source1"),
    Some(&Offset::Sequence(10))
  );
}

#[test]
fn test_tracker_commit_all() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  tracker.record("source1", Offset::Sequence(10)).unwrap();
  tracker.record("source2", Offset::Sequence(20)).unwrap();

  tracker.commit_all().unwrap();

  let committed = tracker.get_all_committed().unwrap();
  assert_eq!(committed.get("source1"), Some(&Offset::Sequence(10)));
  assert_eq!(committed.get("source2"), Some(&Offset::Sequence(20)));
}

#[test]
fn test_tracker_reset_policy_earliest() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Earliest);

  assert_eq!(tracker.get_offset("unknown").unwrap(), Offset::Earliest);
}

#[test]
fn test_tracker_reset_policy_latest() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Latest);

  assert_eq!(tracker.get_offset("unknown").unwrap(), Offset::Latest);
}

#[test]
fn test_tracker_reset_policy_none() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::None);

  let result = tracker.get_offset("unknown");
  assert!(matches!(result, Err(OffsetError::SourceNotFound(_))));
}

#[test]
fn test_tracker_reset_offset() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);

  tracker.record("source1", Offset::Sequence(100)).unwrap();
  tracker.reset("source1", Offset::Sequence(0)).unwrap();

  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::Sequence(0));
}

#[test]
fn test_tracker_clear() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  tracker.record("source1", Offset::Sequence(100)).unwrap();
  tracker.commit("source1").unwrap();
  tracker.record("source1", Offset::Sequence(200)).unwrap();

  tracker.clear("source1").unwrap();

  // Both committed and pending should be cleared
  assert!(tracker.get_all_committed().unwrap().is_empty());
  assert!(tracker.get_all_pending().unwrap().is_empty());
}

#[test]
fn test_offset_error_display() {
  use std::io;
  let io_err = OffsetError::IoError(io::Error::new(io::ErrorKind::NotFound, "file not found"));
  assert!(io_err.to_string().contains("IO error"));

  let ser_err = OffsetError::SerializationError("bad json".to_string());
  assert!(ser_err.to_string().contains("Serialization error"));

  let not_found = OffsetError::SourceNotFound("src".to_string());
  assert!(not_found.to_string().contains("Source not found"));

  let lock_err = OffsetError::LockError("poisoned".to_string());
  assert!(lock_err.to_string().contains("Lock error"));

  let invalid = OffsetError::InvalidOffset("bad format".to_string());
  assert!(invalid.to_string().contains("Invalid offset"));
}

#[test]
fn test_tracker_strategy_getters() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(5))
    .with_reset_policy(OffsetResetPolicy::Latest);

  assert_eq!(tracker.strategy(), CommitStrategy::Periodic(5));
  assert_eq!(tracker.reset_policy(), OffsetResetPolicy::Latest);
}

#[test]
fn test_offset_constructors() {
  let seq = Offset::sequence(42);
  assert_eq!(seq, Offset::Sequence(42));

  let ts = chrono::Utc::now();
  let ts_offset = Offset::timestamp(ts);
  assert!(matches!(ts_offset, Offset::Timestamp(_)));

  let custom = Offset::custom("my-offset");
  assert_eq!(custom, Offset::Custom("my-offset".to_string()));
}

#[test]
fn test_file_store_path() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("offsets.json");

  let store = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store.path(), path);
}

#[test]
fn test_in_memory_store_clone() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::Sequence(10)).unwrap();

  let cloned = store.clone();

  // Both should see the same data (shared state)
  assert_eq!(cloned.get("source1").unwrap(), Some(Offset::Sequence(10)));

  // Update through one should be visible through the other
  cloned.commit("source1", Offset::Sequence(20)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::Sequence(20)));
}

#[test]
fn test_offset_timestamp_display() {
  let ts = chrono::Utc::now();
  let offset = Offset::timestamp(ts);
  let display = offset.to_string();
  assert!(display.starts_with("ts:"));
}

#[test]
fn test_offset_serialization() {
  use serde_json;

  let seq = Offset::Sequence(42);
  let json = serde_json::to_string(&seq).unwrap();
  let deserialized: Offset = serde_json::from_str(&json).unwrap();
  assert_eq!(seq, deserialized);

  let custom = Offset::Custom("test".to_string());
  let json = serde_json::to_string(&custom).unwrap();
  let deserialized: Offset = serde_json::from_str(&json).unwrap();
  assert_eq!(custom, deserialized);

  let earliest = Offset::Earliest;
  let json = serde_json::to_string(&earliest).unwrap();
  let deserialized: Offset = serde_json::from_str(&json).unwrap();
  assert_eq!(earliest, deserialized);

  let latest = Offset::Latest;
  let json = serde_json::to_string(&latest).unwrap();
  let deserialized: Offset = serde_json::from_str(&json).unwrap();
  assert_eq!(latest, deserialized);
}

#[test]
fn test_file_store_empty_file() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("empty.json");

  // Create empty file
  std::fs::write(&path, "").unwrap();

  let store = FileOffsetStore::new(&path).unwrap();
  assert!(store.get("source1").unwrap().is_none());
}

#[test]
fn test_file_store_invalid_json() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("invalid.json");

  // Create invalid JSON
  std::fs::write(&path, "invalid json").unwrap();

  let result = FileOffsetStore::new(&path);
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    OffsetError::SerializationError(_)
  ));
}

#[test]
fn test_tracker_periodic_multiple_sources() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(3));

  tracker.record("source1", Offset::Sequence(1)).unwrap();
  tracker.record("source2", Offset::Sequence(10)).unwrap();
  tracker.record("source1", Offset::Sequence(2)).unwrap();
  tracker.record("source2", Offset::Sequence(11)).unwrap();

  // Neither should be committed yet
  assert!(tracker.get_all_committed().unwrap().is_empty());

  tracker.record("source1", Offset::Sequence(3)).unwrap();

  // source1 should be committed, source2 not yet
  let committed = tracker.get_all_committed().unwrap();
  assert_eq!(committed.get("source1"), Some(&Offset::Sequence(3)));
  assert!(committed.get("source2").is_none());

  tracker.record("source2", Offset::Sequence(12)).unwrap();

  // Now source2 should be committed
  let committed = tracker.get_all_committed().unwrap();
  assert_eq!(committed.get("source2"), Some(&Offset::Sequence(12)));
}

#[test]
fn test_tracker_commit_nonexistent_source() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);

  // Committing a source that doesn't exist should not error
  let result = tracker.commit("nonexistent");
  assert!(result.is_ok());
}

#[test]
fn test_tracker_commit_all_empty() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  // commit_all with no pending should not error
  let result = tracker.commit_all();
  assert!(result.is_ok());
}

#[test]
fn test_tracker_get_offset_existing() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);

  tracker.record("source1", Offset::Sequence(100)).unwrap();
  let offset = tracker.get_offset("source1").unwrap();
  assert_eq!(offset, Offset::Sequence(100));
}

#[test]
fn test_offset_reset_policy_enum() {
  assert_eq!(OffsetResetPolicy::default(), OffsetResetPolicy::Earliest);
  assert_eq!(CommitStrategy::default(), CommitStrategy::Auto);
}

#[test]
fn test_file_store_persist_on_commit() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("persist.json");

  let store1 = FileOffsetStore::new(&path).unwrap();
  store1.commit("source1", Offset::Sequence(100)).unwrap();

  // Create new store instance - should load persisted data
  let store2 = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store2.get("source1").unwrap(), Some(Offset::Sequence(100)));
}

#[test]
fn test_file_store_clear_all_persistence() {
  let dir = tempdir().unwrap();
  let path = dir.path().join("clear_all.json");

  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::Sequence(10)).unwrap();
  store.commit("source2", Offset::Sequence(20)).unwrap();
  store.clear_all().unwrap();

  // Verify persistence
  let store2 = FileOffsetStore::new(&path).unwrap();
  assert!(store2.get_all().unwrap().is_empty());
}

#[test]
fn test_offset_error_from_io_error() {
  use std::io;
  let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
  let offset_err: OffsetError = io_err.into();

  assert!(matches!(offset_err, OffsetError::IoError(_)));
  assert!(offset_err.to_string().contains("IO error"));
}

#[test]
fn test_offset_error_std_error() {
  use std::error::Error;
  let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
  let offset_err: OffsetError = io_err.into();

  // Test that it implements Error trait
  let _: &dyn Error = &offset_err;
  assert!(true);
}
