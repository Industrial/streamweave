//! # Offset Tracking for Resumable Pipeline Processing
//!
//! This module provides comprehensive abstractions for tracking processing offsets,
//! enabling pipelines to resume from where they left off after restarts, crashes,
//! or planned shutdowns. Offset tracking is essential for exactly-once processing
//! semantics and reliable stream processing.
//!
//! # Key Concepts
//!
//! - **Offset**: A position marker in a data stream (sequence number, timestamp, or custom value)
//! - **Offset Store**: Backend storage for persisting offsets (in-memory or file-based)
//! - **Offset Tracker**: High-level interface for managing offsets with configurable commit strategies
//! - **Commit Strategy**: Policy for when offsets are persisted (auto, periodic, or manual)
//! - **Reset Policy**: Behavior when no committed offset is found (earliest, latest, or fail)
//!
//! # Core Types
//!
//! - **[`Offset`]**: Represents a processing offset (Sequence, Timestamp, Custom, Earliest, or Latest)
//! - **[`OffsetStore`]**: Trait for offset storage backends
//! - **[`OffsetTracker`]**: High-level offset management with commit strategies
//! - **[`InMemoryOffsetStore`]**: In-memory offset storage (useful for testing)
//! - **[`FileOffsetStore`]**: File-based offset storage (persists to JSON)
//! - **[`OffsetResetPolicy`]**: Policy for handling missing offsets
//! - **[`CommitStrategy`]**: Strategy for when to commit offsets
//!
//! # Quick Start
//!
//! ## Basic Usage with In-Memory Store
//!
//! ```rust
//! use crate::offset::{Offset, OffsetTracker, InMemoryOffsetStore, CommitStrategy};
//!
//! // Create an in-memory offset store
//! let store = Box::new(InMemoryOffsetStore::new());
//!
//! // Create a tracker with auto-commit strategy
//! let tracker = OffsetTracker::new(store);
//!
//! // Record that we've processed up to sequence 100
//! tracker.record("my_source", Offset::sequence(100))?;
//!
//! // Retrieve the last committed offset
//! let offset = tracker.get_offset("my_source")?;
//! assert_eq!(offset, Offset::sequence(100));
//! # Ok::<(), crate::offset::OffsetError>(())
//! ```
//!
//! ## File-Based Persistence
//!
//! ```rust
//! use crate::offset::{Offset, OffsetTracker, FileOffsetStore, CommitStrategy};
//! use std::path::PathBuf;
//!
//! // Create a file-based offset store
//! let store = Box::new(FileOffsetStore::new("/tmp/offsets.json")?);
//!
//! // Create a tracker with periodic commit (every 10 items)
//! let tracker = OffsetTracker::with_strategy(
//!     store,
//!     CommitStrategy::Periodic(10)
//! );
//!
//! // Record offsets (will commit after 10 records)
//! for i in 0..15 {
//!     tracker.record("source1", Offset::sequence(i))?;
//! }
//!
//! // Manually commit pending offsets
//! tracker.commit("source1")?;
//! # Ok::<(), crate::offset::OffsetError>(())
//! ```
//!
//! ## Offset Reset Policies
//!
//! ```rust
//! use crate::offset::{Offset, OffsetTracker, InMemoryOffsetStore, OffsetResetPolicy};
//!
//! let store = Box::new(InMemoryOffsetStore::new());
//!
//! // Create tracker that starts from earliest if no offset found
//! let tracker = OffsetTracker::new(store)
//!     .with_reset_policy(OffsetResetPolicy::Earliest);
//!
//! // Get offset for new source (will return Earliest due to policy)
//! let offset = tracker.get_offset("new_source")?;
//! assert_eq!(offset, Offset::Earliest);
//! # Ok::<(), crate::offset::OffsetError>(())
//! ```
//!
//! # Commit Strategies
//!
//! The module supports three commit strategies:
//!
//! - **Auto**: Commits immediately after each `record()` call (safest, highest overhead)
//! - **Periodic**: Commits after N records (balance between safety and performance)
//! - **Manual**: Only commits when `commit()` is explicitly called (highest performance, requires careful management)
//!
//! # Offset Types
//!
//! Offsets can be represented in several ways:
//!
//! - **Sequence**: Monotonically increasing sequence number (e.g., Kafka partition offset)
//! - **Timestamp**: UTC timestamp indicating when data was processed
//! - **Custom**: User-defined string identifier (e.g., file path, database row ID)
//! - **Earliest**: Represents the beginning of a stream
//! - **Latest**: Represents the end of a stream (current position)
//!
//! # Integration with StreamWeave
//!
//! Offset tracking integrates seamlessly with StreamWeave's message model:
//!
//! - Offsets are typically stored per source/partition
//! - Message IDs can be used as offsets for exactly-once processing
//! - Offset tracking works with both in-process and distributed execution
//! - File-based stores enable recovery after restarts

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
///
/// File-based offset store implementation.
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

  /// Test-only method to access the pending map for testing.
  #[cfg(test)]
  pub fn get_pending_for_test(&self) -> Arc<RwLock<HashMap<String, (Offset, usize)>>> {
    self.pending.clone()
  }
}
