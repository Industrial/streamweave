use crate::offset::{
  CommitStrategy, FileOffsetStore, InMemoryOffsetStore, Offset, OffsetError, OffsetResetPolicy,
  OffsetStore, OffsetTracker,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tempfile::TempDir;

// ============================================================================
// Offset Enum Tests
// ============================================================================

#[test]
fn test_offset_default() {
  let offset = Offset::default();
  assert!(offset.is_earliest());
}

#[test]
fn test_offset_sequence() {
  let offset = Offset::sequence(42);
  assert!(matches!(offset, Offset::Sequence(42)));
}

#[test]
fn test_offset_timestamp() {
  let ts = Utc::now();
  let offset = Offset::timestamp(ts);
  assert!(matches!(offset, Offset::Timestamp(t) if t == ts));
}

#[test]
fn test_offset_custom() {
  let offset = Offset::custom("my-offset");
  assert!(matches!(offset, Offset::Custom(ref s) if s == "my-offset"));
}

#[test]
fn test_offset_custom_string() {
  let offset = Offset::custom(String::from("test-offset"));
  assert!(matches!(offset, Offset::Custom(ref s) if s == "test-offset"));
}

#[test]
fn test_offset_increment_sequence() {
  let offset = Offset::sequence(5);
  let incremented = offset.increment();
  assert_eq!(incremented, Some(Offset::sequence(6)));
}

#[test]
fn test_offset_increment_timestamp() {
  let ts = Utc::now();
  let offset = Offset::timestamp(ts);
  assert_eq!(offset.increment(), None);
}

#[test]
fn test_offset_increment_custom() {
  let offset = Offset::custom("test");
  assert_eq!(offset.increment(), None);
}

#[test]
fn test_offset_increment_earliest() {
  let offset = Offset::Earliest;
  assert_eq!(offset.increment(), None);
}

#[test]
fn test_offset_increment_latest() {
  let offset = Offset::Latest;
  assert_eq!(offset.increment(), None);
}

#[test]
fn test_offset_is_earliest() {
  assert!(Offset::Earliest.is_earliest());
  assert!(!Offset::Latest.is_earliest());
  assert!(!Offset::sequence(1).is_earliest());
}

#[test]
fn test_offset_is_latest() {
  assert!(Offset::Latest.is_latest());
  assert!(!Offset::Earliest.is_latest());
  assert!(!Offset::sequence(1).is_latest());
}

#[test]
fn test_offset_display_sequence() {
  let offset = Offset::sequence(42);
  assert_eq!(format!("{}", offset), "seq:42");
}

#[test]
fn test_offset_display_timestamp() {
  let ts = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap();
  let offset = Offset::timestamp(ts.with_timezone(&Utc));
  let formatted = format!("{}", offset);
  assert!(formatted.starts_with("ts:2024-01-01"));
}

#[test]
fn test_offset_display_custom() {
  let offset = Offset::custom("my-offset");
  assert_eq!(format!("{}", offset), "custom:my-offset");
}

#[test]
fn test_offset_display_earliest() {
  let offset = Offset::Earliest;
  assert_eq!(format!("{}", offset), "earliest");
}

#[test]
fn test_offset_display_latest() {
  let offset = Offset::Latest;
  assert_eq!(format!("{}", offset), "latest");
}

#[test]
fn test_offset_clone() {
  let offset = Offset::sequence(42);
  let cloned = offset.clone();
  assert_eq!(offset, cloned);
}

#[test]
fn test_offset_eq() {
  assert_eq!(Offset::sequence(1), Offset::sequence(1));
  assert_ne!(Offset::sequence(1), Offset::sequence(2));
  assert_eq!(Offset::Earliest, Offset::Earliest);
  assert_eq!(Offset::Latest, Offset::Latest);
}

#[test]
fn test_offset_hash() {
  use std::collections::HashSet;
  let mut set = HashSet::new();
  set.insert(Offset::sequence(1));
  set.insert(Offset::sequence(2));
  set.insert(Offset::Earliest);
  assert_eq!(set.len(), 3);
}

#[test]
fn test_offset_serde_roundtrip() {
  let offsets = vec![
    Offset::sequence(42),
    Offset::timestamp(Utc::now()),
    Offset::custom("test"),
    Offset::Earliest,
    Offset::Latest,
  ];

  for offset in offsets {
    let serialized = serde_json::to_string(&offset).unwrap();
    let deserialized: Offset = serde_json::from_str(&serialized).unwrap();
    assert_eq!(offset, deserialized);
  }
}

// ============================================================================
// OffsetResetPolicy Tests
// ============================================================================

#[test]
fn test_offset_reset_policy_default() {
  let policy = OffsetResetPolicy::default();
  assert_eq!(policy, OffsetResetPolicy::Earliest);
}

#[test]
fn test_offset_reset_policy_eq() {
  assert_eq!(OffsetResetPolicy::Earliest, OffsetResetPolicy::Earliest);
  assert_ne!(OffsetResetPolicy::Earliest, OffsetResetPolicy::Latest);
}

#[test]
fn test_offset_reset_policy_clone() {
  let policy = OffsetResetPolicy::Latest;
  let cloned = policy;
  assert_eq!(policy, cloned);
}

#[test]
fn test_offset_reset_policy_serde_roundtrip() {
  let policies = vec![
    OffsetResetPolicy::Earliest,
    OffsetResetPolicy::Latest,
    OffsetResetPolicy::None,
  ];

  for policy in policies {
    let serialized = serde_json::to_string(&policy).unwrap();
    let deserialized: OffsetResetPolicy = serde_json::from_str(&serialized).unwrap();
    assert_eq!(policy, deserialized);
  }
}

// ============================================================================
// CommitStrategy Tests
// ============================================================================

#[test]
fn test_commit_strategy_default() {
  let strategy = CommitStrategy::default();
  assert_eq!(strategy, CommitStrategy::Auto);
}

#[test]
fn test_commit_strategy_eq() {
  assert_eq!(CommitStrategy::Auto, CommitStrategy::Auto);
  assert_eq!(CommitStrategy::Periodic(5), CommitStrategy::Periodic(5));
  assert_ne!(CommitStrategy::Periodic(5), CommitStrategy::Periodic(10));
  assert_eq!(CommitStrategy::Manual, CommitStrategy::Manual);
}

#[test]
fn test_commit_strategy_clone() {
  let strategy = CommitStrategy::Periodic(10);
  let cloned = strategy;
  assert_eq!(strategy, cloned);
}

#[test]
fn test_commit_strategy_serde_roundtrip() {
  let strategies = vec![
    CommitStrategy::Auto,
    CommitStrategy::Periodic(5),
    CommitStrategy::Manual,
  ];

  for strategy in strategies {
    let serialized = serde_json::to_string(&strategy).unwrap();
    let deserialized: CommitStrategy = serde_json::from_str(&serialized).unwrap();
    assert_eq!(strategy, deserialized);
  }
}

// ============================================================================
// OffsetError Tests
// ============================================================================

#[test]
fn test_offset_error_display_io() {
  let err = OffsetError::IoError(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "file not found",
  ));
  assert!(format!("{}", err).contains("IO error"));
}

#[test]
fn test_offset_error_display_serialization() {
  let err = OffsetError::SerializationError("invalid json".to_string());
  assert!(format!("{}", err).contains("Serialization error"));
}

#[test]
fn test_offset_error_display_source_not_found() {
  let err = OffsetError::SourceNotFound("source1".to_string());
  assert!(format!("{}", err).contains("Source not found"));
}

#[test]
fn test_offset_error_display_lock_error() {
  let err = OffsetError::LockError("poisoned lock".to_string());
  assert!(format!("{}", err).contains("Lock error"));
}

#[test]
fn test_offset_error_display_invalid_offset() {
  let err = OffsetError::InvalidOffset("bad format".to_string());
  assert!(format!("{}", err).contains("Invalid offset"));
}

#[test]
fn test_offset_error_from_io_error() {
  let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
  let offset_err: OffsetError = io_err.into();
  assert!(matches!(offset_err, OffsetError::IoError(_)));
}

// ============================================================================
// InMemoryOffsetStore Tests
// ============================================================================

#[test]
fn test_in_memory_offset_store_new() {
  let store = InMemoryOffsetStore::new();
  assert!(store.get("test").unwrap().is_none());
}

#[test]
fn test_in_memory_offset_store_with_offsets() {
  let mut offsets = HashMap::new();
  offsets.insert("source1".to_string(), Offset::sequence(5));
  offsets.insert("source2".to_string(), Offset::sequence(10));

  let store = InMemoryOffsetStore::with_offsets(offsets.clone());
  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(5)));
  assert_eq!(store.get("source2").unwrap(), Some(Offset::sequence(10)));
}

#[test]
fn test_in_memory_offset_store_get() {
  let store = InMemoryOffsetStore::new();
  assert_eq!(store.get("nonexistent").unwrap(), None);
}

#[test]
fn test_in_memory_offset_store_commit() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(5)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(5)));
}

#[test]
fn test_in_memory_offset_store_commit_overwrite() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.commit("source1", Offset::sequence(10)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(10)));
}

#[test]
fn test_in_memory_offset_store_get_all() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.commit("source2", Offset::sequence(10)).unwrap();

  let all = store.get_all().unwrap();
  assert_eq!(all.len(), 2);
  assert_eq!(all.get("source1"), Some(&Offset::sequence(5)));
  assert_eq!(all.get("source2"), Some(&Offset::sequence(10)));
}

#[test]
fn test_in_memory_offset_store_clear() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.clear("source1").unwrap();
  assert_eq!(store.get("source1").unwrap(), None);
}

#[test]
fn test_in_memory_offset_store_clear_all() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.commit("source2", Offset::sequence(10)).unwrap();
  store.clear_all().unwrap();
  assert_eq!(store.get_all().unwrap().len(), 0);
}

#[test]
fn test_in_memory_offset_store_clone() {
  let store = InMemoryOffsetStore::new();
  store.commit("source1", Offset::sequence(5)).unwrap();
  let cloned = store.clone();
  assert_eq!(cloned.get("source1").unwrap(), Some(Offset::sequence(5)));
}

// ============================================================================
// FileOffsetStore Tests
// ============================================================================

#[test]
fn test_file_offset_store_new() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  let store = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store.path(), path);
  assert!(store.get("test").unwrap().is_none());
}

#[test]
fn test_file_offset_store_commit_and_get() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::sequence(5)).unwrap();
  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(5)));
}

#[test]
fn test_file_offset_store_persistence() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  {
    let store = FileOffsetStore::new(&path).unwrap();
    store.commit("source1", Offset::sequence(5)).unwrap();
  }

  // Create a new store from the same file
  let store2 = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store2.get("source1").unwrap(), Some(Offset::sequence(5)));
}

#[test]
fn test_file_offset_store_get_all() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.commit("source2", Offset::sequence(10)).unwrap();

  let all = store.get_all().unwrap();
  assert_eq!(all.len(), 2);
}

#[test]
fn test_file_offset_store_clear() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.clear("source1").unwrap();
  assert_eq!(store.get("source1").unwrap(), None);
}

#[test]
fn test_file_offset_store_clear_all() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  let store = FileOffsetStore::new(&path).unwrap();
  store.commit("source1", Offset::sequence(5)).unwrap();
  store.clear_all().unwrap();
  assert_eq!(store.get_all().unwrap().len(), 0);
}

#[test]
fn test_file_offset_store_empty_file() {
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  std::fs::write(&path, "").unwrap();

  let store = FileOffsetStore::new(&path).unwrap();
  assert_eq!(store.get_all().unwrap().len(), 0);
}

// ============================================================================
// OffsetTracker Tests
// ============================================================================

#[test]
fn test_offset_tracker_new() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);
  assert_eq!(tracker.strategy(), CommitStrategy::Auto);
  assert_eq!(tracker.reset_policy(), OffsetResetPolicy::Earliest);
}

#[test]
fn test_offset_tracker_with_strategy() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  assert_eq!(tracker.strategy(), CommitStrategy::Manual);
}

#[test]
fn test_offset_tracker_with_reset_policy() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Latest);
  assert_eq!(tracker.reset_policy(), OffsetResetPolicy::Latest);
}

#[test]
fn test_offset_tracker_get_offset_existing() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);
  tracker.reset("source1", Offset::sequence(5)).unwrap();
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));
}

#[test]
fn test_offset_tracker_get_offset_earliest_policy() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Earliest);
  assert_eq!(tracker.get_offset("nonexistent").unwrap(), Offset::Earliest);
}

#[test]
fn test_offset_tracker_get_offset_latest_policy() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::Latest);
  assert_eq!(tracker.get_offset("nonexistent").unwrap(), Offset::Latest);
}

#[test]
fn test_offset_tracker_get_offset_none_policy() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store).with_reset_policy(OffsetResetPolicy::None);
  assert!(tracker.get_offset("nonexistent").is_err());
}

#[test]
fn test_offset_tracker_record_auto() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));
}

#[test]
fn test_offset_tracker_record_periodic() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(3));
  tracker.record("source1", Offset::sequence(1)).unwrap();
  tracker.record("source1", Offset::sequence(2)).unwrap();
  // Should not be committed yet
  assert!(tracker.get_offset("source1").unwrap().is_earliest());
  tracker.record("source1", Offset::sequence(3)).unwrap();
  // Should be committed now
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(3));
}

#[test]
fn test_offset_tracker_record_manual() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  // Should not be committed yet
  assert!(tracker.get_offset("source1").unwrap().is_earliest());
  tracker.commit("source1").unwrap();
  // Should be committed now
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));
}

#[test]
fn test_offset_tracker_commit() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  tracker.commit("source1").unwrap();
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));
}

#[test]
fn test_offset_tracker_commit_all() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  tracker.record("source2", Offset::sequence(10)).unwrap();
  tracker.commit_all().unwrap();
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));
  assert_eq!(tracker.get_offset("source2").unwrap(), Offset::sequence(10));
}

#[test]
fn test_offset_tracker_reset() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);
  tracker.reset("source1", Offset::sequence(5)).unwrap();
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));
}

#[test]
fn test_offset_tracker_clear() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  tracker.clear("source1").unwrap();
  assert!(tracker.get_offset("source1").unwrap().is_earliest());
}

#[test]
fn test_offset_tracker_get_all_committed() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::new(store);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  tracker.record("source2", Offset::sequence(10)).unwrap();
  let committed = tracker.get_all_committed().unwrap();
  assert_eq!(committed.len(), 2);
}

#[test]
fn test_offset_tracker_get_all_pending() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  tracker.record("source2", Offset::sequence(10)).unwrap();
  let pending = tracker.get_all_pending().unwrap();
  assert_eq!(pending.len(), 2);
  assert_eq!(pending.get("source1"), Some(&Offset::sequence(5)));
}

#[test]
fn test_offset_tracker_periodic_counter_reset() {
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(2));
  tracker.record("source1", Offset::sequence(1)).unwrap();
  tracker.record("source1", Offset::sequence(2)).unwrap();
  // Counter should reset after commit
  tracker.record("source1", Offset::sequence(3)).unwrap();
  // Should not commit yet (counter = 1)
  tracker.record("source1", Offset::sequence(4)).unwrap();
  // Should commit now (counter = 2)
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(4));
}
