use crate::offset::{
  CommitStrategy, FileOffsetStore, InMemoryOffsetStore, Offset, OffsetError, OffsetResetPolicy,
  OffsetStore, OffsetTracker,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::io;
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

#[test]
fn test_in_memory_offset_store_default() {
  // Test Default implementation explicitly
  let store = InMemoryOffsetStore::default();
  assert!(store.get("test").unwrap().is_none());
  assert_eq!(store.get_all().unwrap().len(), 0);
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

// ============================================================================
// Error Path and Edge Case Tests
// ============================================================================

/// Mock OffsetStore that can return errors for testing error paths
#[derive(Debug)]
struct MockErrorStore {
  should_error_on_get: bool,
  should_error_on_commit: bool,
  should_error_on_get_all: bool,
  should_error_on_clear: bool,
  should_error_on_clear_all: bool,
}

impl MockErrorStore {
  fn new() -> Self {
    Self {
      should_error_on_get: false,
      should_error_on_commit: false,
      should_error_on_get_all: false,
      should_error_on_clear: false,
      should_error_on_clear_all: false,
    }
  }

  fn with_get_error(mut self) -> Self {
    self.should_error_on_get = true;
    self
  }

  fn with_commit_error(mut self) -> Self {
    self.should_error_on_commit = true;
    self
  }

  fn with_get_all_error(mut self) -> Self {
    self.should_error_on_get_all = true;
    self
  }

  fn with_clear_error(mut self) -> Self {
    self.should_error_on_clear = true;
    self
  }

  #[allow(dead_code)]
  fn with_clear_all_error(mut self) -> Self {
    self.should_error_on_clear_all = true;
    self
  }
}

impl OffsetStore for MockErrorStore {
  fn get(&self, _source: &str) -> Result<Option<Offset>, OffsetError> {
    if self.should_error_on_get {
      Err(OffsetError::IoError(io::Error::other("Mock get error")))
    } else {
      Ok(None)
    }
  }

  fn commit(&self, _source: &str, _offset: Offset) -> Result<(), OffsetError> {
    if self.should_error_on_commit {
      Err(OffsetError::IoError(io::Error::other("Mock commit error")))
    } else {
      Ok(())
    }
  }

  fn get_all(&self) -> Result<HashMap<String, Offset>, OffsetError> {
    if self.should_error_on_get_all {
      Err(OffsetError::IoError(io::Error::other("Mock get_all error")))
    } else {
      Ok(HashMap::new())
    }
  }

  fn clear(&self, _source: &str) -> Result<(), OffsetError> {
    if self.should_error_on_clear {
      Err(OffsetError::IoError(io::Error::other("Mock clear error")))
    } else {
      Ok(())
    }
  }

  fn clear_all(&self) -> Result<(), OffsetError> {
    if self.should_error_on_clear_all {
      Err(OffsetError::IoError(io::Error::other(
        "Mock clear_all error",
      )))
    } else {
      Ok(())
    }
  }
}

#[test]
fn test_offset_tracker_get_offset_error() {
  let store = Box::new(MockErrorStore::new().with_get_error());
  let tracker = OffsetTracker::new(store);
  let result = tracker.get_offset("source1");
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_record_auto_error() {
  let store = Box::new(MockErrorStore::new().with_commit_error());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Auto);
  let result = tracker.record("source1", Offset::sequence(5));
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_record_periodic_error() {
  let store = Box::new(MockErrorStore::new().with_commit_error());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(1));
  // First record should trigger commit (interval = 1)
  let result = tracker.record("source1", Offset::sequence(5));
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_commit_error() {
  let store = Box::new(MockErrorStore::new().with_commit_error());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  let result = tracker.commit("source1");
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_commit_all_error() {
  let store = Box::new(MockErrorStore::new().with_commit_error());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  tracker.record("source1", Offset::sequence(5)).unwrap();
  let result = tracker.commit_all();
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_reset_error() {
  let store = Box::new(MockErrorStore::new().with_commit_error());
  let tracker = OffsetTracker::new(store);
  let result = tracker.reset("source1", Offset::sequence(5));
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_clear_error() {
  let store = Box::new(MockErrorStore::new().with_clear_error());
  let tracker = OffsetTracker::new(store);
  let result = tracker.clear("source1");
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_get_all_committed_error() {
  let store = Box::new(MockErrorStore::new().with_get_all_error());
  let tracker = OffsetTracker::new(store);
  let result = tracker.get_all_committed();
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_offset_tracker_commit_no_pending() {
  // Commit when there's no pending offset should succeed (no-op)
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  let result = tracker.commit("nonexistent");
  assert!(result.is_ok());
}

#[test]
fn test_offset_tracker_commit_entry_removed() {
  // Test the edge case where an entry might be removed between
  // reading the pending offset and getting mutable access
  // This tests the None branch in pending.get_mut(source) at line 446
  // In practice, this is rare but can happen in concurrent scenarios
  // For testing, we simulate this by clearing the entry after commit
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  // Record an offset
  tracker.record("source1", Offset::sequence(5)).unwrap();

  // Commit - this should succeed and reset the counter
  tracker.commit("source1").unwrap();

  // Verify it was committed
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(5));

  // Now commit again - this time, if the entry was somehow removed,
  // pending.get_mut would return None, but commit should still succeed
  // because pending_offset is None (the entry was cleared)
  // To test the actual None branch at line 446, we'd need the entry
  // to exist during read but not during write, which is hard to simulate
  // in single-threaded tests. But we can at least test that commit
  // handles the case gracefully.
  let result = tracker.commit("nonexistent");
  assert!(result.is_ok());
}

#[test]
fn test_offset_tracker_get_pending_for_test() {
  // Test the test helper method to access the pending map
  // This covers lines 516-517
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  // Use the test helper to get the pending map
  let pending = tracker.get_pending_for_test();

  // Verify we can access it
  let pending_read = pending.read().unwrap();
  assert!(pending_read.is_empty());
  drop(pending_read);

  // Record an offset
  tracker.record("source1", Offset::sequence(5)).unwrap();

  // Verify we can see it in the pending map via the test helper
  let pending_read = pending.read().unwrap();
  assert_eq!(pending_read.len(), 1);
  assert_eq!(pending_read.get("source1").unwrap().0, Offset::sequence(5));
}

#[test]
fn test_offset_tracker_commit_all_empty() {
  // Commit all when there are no pending offsets should succeed
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  let result = tracker.commit_all();
  assert!(result.is_ok());
}

#[test]
fn test_offset_tracker_get_all_pending_empty() {
  // Get all pending when there are no pending offsets should return empty map
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);
  let pending = tracker.get_all_pending().unwrap();
  assert!(pending.is_empty());
}

#[test]
fn test_offset_tracker_record_periodic_exact_interval() {
  // Test that periodic commit happens exactly at the interval
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(3));

  // Record 3 items - should trigger commit
  tracker.record("source1", Offset::sequence(1)).unwrap();
  tracker.record("source1", Offset::sequence(2)).unwrap();
  tracker.record("source1", Offset::sequence(3)).unwrap();

  // Verify it was committed
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(3));

  // Counter should be reset, next record should not commit yet
  tracker.record("source1", Offset::sequence(4)).unwrap();
  // Verify it's still the same (not committed yet)
  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(3));
}

#[test]
fn test_offset_tracker_record_periodic_multiple_sources() {
  // Test periodic commit with multiple sources
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Periodic(2));

  // Record for source1
  tracker.record("source1", Offset::sequence(1)).unwrap();
  tracker.record("source1", Offset::sequence(2)).unwrap(); // Should commit

  // Record for source2
  tracker.record("source2", Offset::sequence(10)).unwrap();
  tracker.record("source2", Offset::sequence(20)).unwrap(); // Should commit

  assert_eq!(tracker.get_offset("source1").unwrap(), Offset::sequence(2));
  assert_eq!(tracker.get_offset("source2").unwrap(), Offset::sequence(20));
}

#[test]
fn test_offset_tracker_clear_removes_pending() {
  // Test that clear removes both committed and pending offsets
  let store = Box::new(InMemoryOffsetStore::new());
  let tracker = OffsetTracker::with_strategy(store, CommitStrategy::Manual);

  // Record a pending offset
  tracker.record("source1", Offset::sequence(5)).unwrap();
  assert_eq!(tracker.get_all_pending().unwrap().len(), 1);

  // Clear it
  tracker.clear("source1").unwrap();

  // Verify pending is removed
  assert_eq!(tracker.get_all_pending().unwrap().len(), 0);

  // Verify committed is also cleared
  assert!(tracker.get_offset("source1").unwrap().is_earliest());
}

#[test]
fn test_file_offset_store_invalid_json() {
  // Test FileOffsetStore with invalid JSON
  let temp_dir = TempDir::new().unwrap();
  let file_path = temp_dir.path().join("offsets.json");

  // Write invalid JSON
  std::fs::write(&file_path, "invalid json").unwrap();

  let result = FileOffsetStore::new(&file_path);
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    OffsetError::SerializationError(_)
  ));
}

#[test]
fn test_file_offset_store_nonexistent_parent() {
  // Test FileOffsetStore when parent directory doesn't exist
  // This should create the parent directory
  let temp_dir = TempDir::new().unwrap();
  let file_path = temp_dir.path().join("subdir").join("offsets.json");

  let store = FileOffsetStore::new(&file_path).unwrap();
  store.commit("source1", Offset::sequence(5)).unwrap();

  // Verify file was created
  assert!(file_path.exists());
  assert!(file_path.parent().unwrap().exists());
}

#[test]
fn test_file_offset_store_path_no_parent() {
  // Test FileOffsetStore with a path that has a parent
  // This ensures the if block in persist executes (lines 268-270)
  let temp_dir = TempDir::new().unwrap();
  let file_path = temp_dir.path().join("subdir").join("offsets.json");
  let store = FileOffsetStore::new(&file_path).unwrap();

  // This should execute the if block (parent is Some) and hit lines 268, 269, and 270
  store.commit("source1", Offset::sequence(5)).unwrap();
  assert!(file_path.exists());
  assert!(file_path.parent().unwrap().exists());

  // Also test persist via clear_all to ensure line 270 is covered
  store.clear_all().unwrap();

  // Test persist via clear as well
  store.commit("source2", Offset::sequence(10)).unwrap();
  store.clear("source2").unwrap();
}

#[test]
fn test_file_offset_store_new_with_directory() {
  // Test FileOffsetStore::new when path points to a directory instead of a file
  // This should fail because fs::read_to_string can't read a directory
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path();

  // Try to create a FileOffsetStore with a directory path
  // This should return an error because read_to_string will fail
  let result = FileOffsetStore::new(dir_path);
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), OffsetError::IoError(_)));
}

#[test]
fn test_file_offset_store_new_with_valid_existing_file() {
  // Test FileOffsetStore::new when file exists with valid JSON
  // This tests the branch where path.exists() is true and data is not empty
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");

  // Write valid JSON to file
  let initial_offsets = serde_json::json!({
    "source1": {"Sequence": 5},
    "source2": {"Sequence": 10}
  });
  std::fs::write(&path, serde_json::to_string(&initial_offsets).unwrap()).unwrap();

  // Create store from existing file
  let store = FileOffsetStore::new(&path).unwrap();

  // Verify offsets were loaded
  assert_eq!(store.get("source1").unwrap(), Some(Offset::sequence(5)));
  assert_eq!(store.get("source2").unwrap(), Some(Offset::sequence(10)));
}

#[test]
fn test_file_offset_store_path_parent_none() {
  // Test FileOffsetStore::persist when path.parent() returns None
  // This happens only for root paths like "/", which we can't write to in tests
  // However, we can use a relative path that has no parent by using just a filename
  // Actually, on Unix, any non-root path has a parent. The only paths without parents
  // are "/" on Unix and "C:\" on Windows, which we can't write to.
  // Since we can't test the None branch directly, we ensure the Some branch is well-covered
  // by testing with paths that have parents (which is the common case).
  // The None branch is defensive code for edge cases that are impractical to test.

  // Test with a relative filename (though it still has a parent ".")
  let temp_dir = TempDir::new().unwrap();
  let file_path = temp_dir.path().join("offsets.json");
  let store = FileOffsetStore::new(&file_path).unwrap();

  // This should hit the Some branch and execute lines 268, 269, and 270
  store.commit("source1", Offset::sequence(5)).unwrap();
  assert!(file_path.exists());
}

#[test]
fn test_file_offset_store_clone() {
  // Test FileOffsetStore::Clone implementation
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("offsets.json");
  let store1 = FileOffsetStore::new(&path).unwrap();
  store1.commit("source1", Offset::sequence(5)).unwrap();

  let store2 = store1.clone();
  assert_eq!(store2.get("source1").unwrap(), Some(Offset::sequence(5)));
  assert_eq!(store2.path(), path);
}
