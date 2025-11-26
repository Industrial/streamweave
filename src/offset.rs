//! Offset tracking for resumable pipeline processing.
//!
//! This module provides abstractions for tracking processing offsets,
//! enabling pipelines to resume from where they left off after restarts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Represents a processing offset.
///
/// Offsets can be sequence numbers, timestamps, or custom values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Offset {
  /// A sequence number offset.
  Sequence(u64),
  /// A timestamp-based offset.
  Timestamp(DateTime<Utc>),
  /// A custom string offset.
  Custom(String),
  /// Represents the beginning of a stream.
  #[default]
  Earliest,
  /// Represents the end of a stream (latest).
  Latest,
}

impl Display for Offset {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Offset::Sequence(n) => write!(f, "seq:{}", n),
      Offset::Timestamp(ts) => write!(f, "ts:{}", ts.to_rfc3339()),
      Offset::Custom(s) => write!(f, "custom:{}", s),
      Offset::Earliest => write!(f, "earliest"),
      Offset::Latest => write!(f, "latest"),
    }
  }
}

impl Offset {
  /// Creates a sequence offset.
  pub fn sequence(n: u64) -> Self {
    Offset::Sequence(n)
  }

  /// Creates a timestamp offset.
  pub fn timestamp(ts: DateTime<Utc>) -> Self {
    Offset::Timestamp(ts)
  }

  /// Creates a custom string offset.
  pub fn custom(s: impl Into<String>) -> Self {
    Offset::Custom(s.into())
  }

  /// Increments a sequence offset by one.
  /// Returns None for non-sequence offsets.
  pub fn increment(&self) -> Option<Self> {
    match self {
      Offset::Sequence(n) => Some(Offset::Sequence(n + 1)),
      _ => None,
    }
  }

  /// Returns true if this is the earliest offset.
  pub fn is_earliest(&self) -> bool {
    matches!(self, Offset::Earliest)
  }

  /// Returns true if this is the latest offset.
  pub fn is_latest(&self) -> bool {
    matches!(self, Offset::Latest)
  }
}

/// Policy for resetting offsets when no committed offset is found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OffsetResetPolicy {
  /// Start from the earliest available offset.
  #[default]
  Earliest,
  /// Start from the latest available offset.
  Latest,
  /// Fail if no offset is found.
  None,
}

/// Strategy for committing offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CommitStrategy {
  /// Automatically commit after each item is processed.
  #[default]
  Auto,
  /// Commit periodically based on count.
  Periodic(usize),
  /// Only commit when explicitly requested.
  Manual,
}

/// Error type for offset operations.
#[derive(Debug)]
pub enum OffsetError {
  /// IO error during persistence.
  IoError(io::Error),
  /// Serialization/deserialization error.
  SerializationError(String),
  /// Source not found.
  SourceNotFound(String),
  /// Lock acquisition failed.
  LockError(String),
  /// Invalid offset format.
  InvalidOffset(String),
}

impl Display for OffsetError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OffsetError::IoError(e) => write!(f, "IO error: {}", e),
      OffsetError::SerializationError(s) => write!(f, "Serialization error: {}", s),
      OffsetError::SourceNotFound(s) => write!(f, "Source not found: {}", s),
      OffsetError::LockError(s) => write!(f, "Lock error: {}", s),
      OffsetError::InvalidOffset(s) => write!(f, "Invalid offset: {}", s),
    }
  }
}

impl std::error::Error for OffsetError {}

impl From<io::Error> for OffsetError {
  fn from(err: io::Error) -> Self {
    OffsetError::IoError(err)
  }
}

/// Result type for offset operations.
pub type OffsetResult<T> = Result<T, OffsetError>;

/// Trait for offset storage backends.
///
/// Implementations of this trait handle persisting and retrieving offsets.
pub trait OffsetStore: Send + Sync + std::fmt::Debug {
  /// Get the committed offset for a source.
  fn get(&self, source: &str) -> OffsetResult<Option<Offset>>;

  /// Commit an offset for a source.
  fn commit(&self, source: &str, offset: Offset) -> OffsetResult<()>;

  /// Get all committed offsets.
  fn get_all(&self) -> OffsetResult<HashMap<String, Offset>>;

  /// Clear the offset for a source.
  fn clear(&self, source: &str) -> OffsetResult<()>;

  /// Clear all offsets.
  fn clear_all(&self) -> OffsetResult<()>;
}

/// In-memory offset store.
///
/// This is useful for testing or scenarios where persistence across restarts
/// is not required.
#[derive(Debug, Clone, Default)]
pub struct InMemoryOffsetStore {
  offsets: Arc<RwLock<HashMap<String, Offset>>>,
}

impl InMemoryOffsetStore {
  /// Creates a new in-memory offset store.
  pub fn new() -> Self {
    Self::default()
  }

  /// Creates an in-memory offset store with initial offsets.
  pub fn with_offsets(offsets: HashMap<String, Offset>) -> Self {
    Self {
      offsets: Arc::new(RwLock::new(offsets)),
    }
  }
}

impl OffsetStore for InMemoryOffsetStore {
  fn get(&self, source: &str) -> OffsetResult<Option<Offset>> {
    let offsets = self
      .offsets
      .read()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    Ok(offsets.get(source).cloned())
  }

  fn commit(&self, source: &str, offset: Offset) -> OffsetResult<()> {
    let mut offsets = self
      .offsets
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    offsets.insert(source.to_string(), offset);
    Ok(())
  }

  fn get_all(&self) -> OffsetResult<HashMap<String, Offset>> {
    let offsets = self
      .offsets
      .read()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    Ok(offsets.clone())
  }

  fn clear(&self, source: &str) -> OffsetResult<()> {
    let mut offsets = self
      .offsets
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    offsets.remove(source);
    Ok(())
  }

  fn clear_all(&self) -> OffsetResult<()> {
    let mut offsets = self
      .offsets
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    offsets.clear();
    Ok(())
  }
}

/// File-based offset store.
///
/// Persists offsets to a JSON file on disk.
#[derive(Debug, Clone)]
pub struct FileOffsetStore {
  path: PathBuf,
  cache: Arc<RwLock<HashMap<String, Offset>>>,
}

impl FileOffsetStore {
  /// Creates a new file-based offset store.
  pub fn new<P: AsRef<Path>>(path: P) -> OffsetResult<Self> {
    let path = path.as_ref().to_path_buf();

    // Load existing offsets if the file exists
    let cache = if path.exists() {
      let data = fs::read_to_string(&path)?;
      if data.is_empty() {
        HashMap::new()
      } else {
        serde_json::from_str(&data).map_err(|e| OffsetError::SerializationError(e.to_string()))?
      }
    } else {
      HashMap::new()
    };

    Ok(Self {
      path,
      cache: Arc::new(RwLock::new(cache)),
    })
  }

  /// Persists the current offsets to disk.
  fn persist(&self, offsets: &HashMap<String, Offset>) -> OffsetResult<()> {
    // Ensure parent directory exists
    if let Some(parent) = self.path.parent() {
      fs::create_dir_all(parent)?;
    }

    let data = serde_json::to_string_pretty(offsets)
      .map_err(|e| OffsetError::SerializationError(e.to_string()))?;
    fs::write(&self.path, data)?;
    Ok(())
  }

  /// Returns the path to the offset file.
  pub fn path(&self) -> &Path {
    &self.path
  }
}

impl OffsetStore for FileOffsetStore {
  fn get(&self, source: &str) -> OffsetResult<Option<Offset>> {
    let cache = self
      .cache
      .read()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    Ok(cache.get(source).cloned())
  }

  fn commit(&self, source: &str, offset: Offset) -> OffsetResult<()> {
    let mut cache = self
      .cache
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    cache.insert(source.to_string(), offset);
    self.persist(&cache)?;
    Ok(())
  }

  fn get_all(&self) -> OffsetResult<HashMap<String, Offset>> {
    let cache = self
      .cache
      .read()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    Ok(cache.clone())
  }

  fn clear(&self, source: &str) -> OffsetResult<()> {
    let mut cache = self
      .cache
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    cache.remove(source);
    self.persist(&cache)?;
    Ok(())
  }

  fn clear_all(&self) -> OffsetResult<()> {
    let mut cache = self
      .cache
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    cache.clear();
    self.persist(&cache)?;
    Ok(())
  }
}

/// Tracks processing offsets with configurable commit strategies.
///
/// The `OffsetTracker` wraps an `OffsetStore` and provides convenient
/// methods for tracking and committing offsets based on the configured
/// strategy.
#[derive(Debug)]
pub struct OffsetTracker {
  store: Box<dyn OffsetStore>,
  strategy: CommitStrategy,
  reset_policy: OffsetResetPolicy,
  pending: Arc<RwLock<HashMap<String, (Offset, usize)>>>,
}

impl OffsetTracker {
  /// Creates a new offset tracker with the given store and default settings.
  pub fn new(store: Box<dyn OffsetStore>) -> Self {
    Self {
      store,
      strategy: CommitStrategy::default(),
      reset_policy: OffsetResetPolicy::default(),
      pending: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  /// Creates a new offset tracker with the specified commit strategy.
  pub fn with_strategy(store: Box<dyn OffsetStore>, strategy: CommitStrategy) -> Self {
    Self {
      store,
      strategy,
      reset_policy: OffsetResetPolicy::default(),
      pending: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  /// Sets the offset reset policy.
  pub fn with_reset_policy(mut self, policy: OffsetResetPolicy) -> Self {
    self.reset_policy = policy;
    self
  }

  /// Gets the current committed offset for a source, applying the reset policy
  /// if no offset is found.
  pub fn get_offset(&self, source: &str) -> OffsetResult<Offset> {
    match self.store.get(source)? {
      Some(offset) => Ok(offset),
      None => match self.reset_policy {
        OffsetResetPolicy::Earliest => Ok(Offset::Earliest),
        OffsetResetPolicy::Latest => Ok(Offset::Latest),
        OffsetResetPolicy::None => Err(OffsetError::SourceNotFound(source.to_string())),
      },
    }
  }

  /// Records that an offset has been processed.
  ///
  /// Based on the commit strategy, this may immediately commit the offset
  /// or hold it for later batch commit.
  pub fn record(&self, source: &str, offset: Offset) -> OffsetResult<()> {
    match self.strategy {
      CommitStrategy::Auto => {
        self.store.commit(source, offset)?;
      }
      CommitStrategy::Periodic(interval) => {
        let mut pending = self
          .pending
          .write()
          .map_err(|e| OffsetError::LockError(e.to_string()))?;

        let entry = pending
          .entry(source.to_string())
          .or_insert((offset.clone(), 0));
        entry.0 = offset;
        entry.1 += 1;

        if entry.1 >= interval {
          let offset_to_commit = entry.0.clone();
          entry.1 = 0;
          drop(pending); // Release lock before committing
          self.store.commit(source, offset_to_commit)?;
        }
      }
      CommitStrategy::Manual => {
        let mut pending = self
          .pending
          .write()
          .map_err(|e| OffsetError::LockError(e.to_string()))?;
        let entry = pending
          .entry(source.to_string())
          .or_insert((offset.clone(), 0));
        entry.0 = offset;
        entry.1 += 1;
      }
    }
    Ok(())
  }

  /// Commits the pending offset for a specific source.
  ///
  /// This is useful for manual commit strategy or when forcing a commit.
  pub fn commit(&self, source: &str) -> OffsetResult<()> {
    let pending_offset = {
      let pending = self
        .pending
        .read()
        .map_err(|e| OffsetError::LockError(e.to_string()))?;
      pending.get(source).map(|(o, _)| o.clone())
    };

    if let Some(offset) = pending_offset {
      self.store.commit(source, offset)?;
      let mut pending = self
        .pending
        .write()
        .map_err(|e| OffsetError::LockError(e.to_string()))?;
      if let Some(entry) = pending.get_mut(source) {
        entry.1 = 0;
      }
    }
    Ok(())
  }

  /// Commits all pending offsets.
  pub fn commit_all(&self) -> OffsetResult<()> {
    let sources: Vec<String> = {
      let pending = self
        .pending
        .read()
        .map_err(|e| OffsetError::LockError(e.to_string()))?;
      pending.keys().cloned().collect()
    };

    for source in sources {
      self.commit(&source)?;
    }
    Ok(())
  }

  /// Resets the offset for a source to the specified value.
  pub fn reset(&self, source: &str, offset: Offset) -> OffsetResult<()> {
    self.store.commit(source, offset)
  }

  /// Clears the offset for a source.
  pub fn clear(&self, source: &str) -> OffsetResult<()> {
    self.store.clear(source)?;
    let mut pending = self
      .pending
      .write()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    pending.remove(source);
    Ok(())
  }

  /// Returns the current commit strategy.
  pub fn strategy(&self) -> CommitStrategy {
    self.strategy
  }

  /// Returns the current reset policy.
  pub fn reset_policy(&self) -> OffsetResetPolicy {
    self.reset_policy
  }

  /// Gets all committed offsets.
  pub fn get_all_committed(&self) -> OffsetResult<HashMap<String, Offset>> {
    self.store.get_all()
  }

  /// Gets all pending offsets (not yet committed).
  pub fn get_all_pending(&self) -> OffsetResult<HashMap<String, Offset>> {
    let pending = self
      .pending
      .read()
      .map_err(|e| OffsetError::LockError(e.to_string()))?;
    Ok(
      pending
        .iter()
        .map(|(k, (o, _))| (k.clone(), o.clone()))
        .collect(),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
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

  // FileOffsetStore tests
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

    let ts = Utc::now();
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
}
