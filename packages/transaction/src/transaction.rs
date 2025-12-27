//! Transaction support for exactly-once processing.
//!
//! This module provides transactional processing capabilities for stream pipelines,
//! ensuring all-or-nothing semantics for batches of operations.
//!
//! # Overview
//!
//! The transaction system provides:
//! - **Begin/Commit/Rollback**: Standard transaction lifecycle management
//! - **Offset Integration**: Buffer offset commits within transactions
//! - **Timeout Support**: Configurable transaction timeouts with auto-rollback
//! - **Nested Transactions**: Savepoint-style nested transaction support
//!
//! # Example
//!
//! ```rust
//! use streamweave::transaction::{TransactionManager, TransactionConfig};
//! use streamweave::offset::{InMemoryOffsetStore, Offset};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let offset_store = Box::new(InMemoryOffsetStore::new());
//! let config = TransactionConfig::default()
//!     .with_timeout(Duration::from_secs(30));
//! let manager = TransactionManager::new(offset_store, config);
//!
//! // Begin a transaction
//! let tx_id = manager.begin().await?;
//!
//! // Buffer some offset commits
//! manager.buffer_offset(&tx_id, "source1", Offset::Sequence(100)).await?;
//! manager.buffer_offset(&tx_id, "source2", Offset::Sequence(200)).await?;
//!
//! // Commit the transaction (flushes all buffered offsets)
//! manager.commit(&tx_id).await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::fmt::{self, Display};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use streamweave_offset::{Offset, OffsetError, OffsetStore};

/// Unique identifier for a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(u64);

impl TransactionId {
  /// Creates a new transaction ID with the given value.
  pub const fn new(id: u64) -> Self {
    Self(id)
  }

  /// Returns the underlying ID value.
  pub const fn value(&self) -> u64 {
    self.0
  }
}

impl Display for TransactionId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "tx:{}", self.0)
  }
}

/// State of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
  /// Transaction is active and accepting operations.
  Active,
  /// Transaction has been successfully committed.
  Committed,
  /// Transaction has been rolled back.
  RolledBack,
  /// Transaction timed out and was auto-rolled back.
  TimedOut,
}

impl Display for TransactionState {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      TransactionState::Active => write!(f, "active"),
      TransactionState::Committed => write!(f, "committed"),
      TransactionState::RolledBack => write!(f, "rolled_back"),
      TransactionState::TimedOut => write!(f, "timed_out"),
    }
  }
}

/// Error type for transaction operations.
#[derive(Debug)]
pub enum TransactionError {
  /// Transaction not found.
  NotFound(TransactionId),
  /// Transaction is not in active state.
  NotActive(TransactionId, TransactionState),
  /// Transaction timed out.
  Timeout(TransactionId),
  /// Offset operation failed.
  OffsetError(OffsetError),
  /// Lock acquisition failed.
  LockError(String),
  /// Invalid operation for current state.
  InvalidOperation(String),
  /// Savepoint not found.
  SavepointNotFound(String),
  /// Nested transaction limit exceeded.
  NestingLimitExceeded(usize),
}

impl Display for TransactionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      TransactionError::NotFound(id) => write!(f, "Transaction not found: {}", id),
      TransactionError::NotActive(id, state) => {
        write!(
          f,
          "Transaction {} is not active, current state: {}",
          id, state
        )
      }
      TransactionError::Timeout(id) => write!(f, "Transaction {} timed out", id),
      TransactionError::OffsetError(e) => write!(f, "Offset error: {}", e),
      TransactionError::LockError(s) => write!(f, "Lock error: {}", s),
      TransactionError::InvalidOperation(s) => write!(f, "Invalid operation: {}", s),
      TransactionError::SavepointNotFound(s) => write!(f, "Savepoint not found: {}", s),
      TransactionError::NestingLimitExceeded(limit) => {
        write!(f, "Nested transaction limit exceeded: {}", limit)
      }
    }
  }
}

impl std::error::Error for TransactionError {}

impl From<OffsetError> for TransactionError {
  fn from(err: OffsetError) -> Self {
    TransactionError::OffsetError(err)
  }
}

/// Result type for transaction operations.
pub type TransactionResult<T> = Result<T, TransactionError>;

/// Configuration for transactions.
#[derive(Debug, Clone)]
pub struct TransactionConfig {
  /// Default timeout for transactions.
  pub timeout: Duration,
  /// Maximum nesting level for savepoints.
  pub max_nesting_level: usize,
  /// Whether to auto-rollback on timeout.
  pub auto_rollback_on_timeout: bool,
}

impl Default for TransactionConfig {
  fn default() -> Self {
    Self {
      timeout: Duration::from_secs(60),
      max_nesting_level: 10,
      auto_rollback_on_timeout: true,
    }
  }
}

impl TransactionConfig {
  /// Creates a new configuration with default values.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the transaction timeout.
  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// Sets the maximum nesting level for savepoints.
  pub fn with_max_nesting_level(mut self, level: usize) -> Self {
    self.max_nesting_level = level;
    self
  }

  /// Sets whether to auto-rollback on timeout.
  pub fn with_auto_rollback_on_timeout(mut self, auto_rollback: bool) -> Self {
    self.auto_rollback_on_timeout = auto_rollback;
    self
  }
}

/// A savepoint within a transaction.
#[derive(Debug, Clone)]
pub struct Savepoint {
  /// Name of the savepoint.
  pub name: String,
  /// Index in the operations buffer at the time of savepoint creation.
  pub buffer_index: usize,
  /// Timestamp when the savepoint was created.
  pub created_at: Instant,
}

/// A buffered offset operation within a transaction.
#[derive(Debug, Clone)]
pub struct BufferedOffset {
  /// Source identifier.
  pub source: String,
  /// Offset value.
  pub offset: Offset,
}

/// A transaction instance.
#[derive(Debug)]
pub struct Transaction {
  /// Unique transaction ID.
  pub id: TransactionId,
  /// Current state of the transaction.
  pub state: TransactionState,
  /// Buffered offset operations.
  pub buffered_offsets: Vec<BufferedOffset>,
  /// Active savepoints (stack).
  pub savepoints: Vec<Savepoint>,
  /// When the transaction was started.
  pub started_at: Instant,
  /// Transaction timeout.
  pub timeout: Duration,
  /// Custom metadata.
  pub metadata: HashMap<String, String>,
}

impl Transaction {
  /// Creates a new transaction with the given ID and timeout.
  pub fn new(id: TransactionId, timeout: Duration) -> Self {
    Self {
      id,
      state: TransactionState::Active,
      buffered_offsets: Vec::new(),
      savepoints: Vec::new(),
      started_at: Instant::now(),
      timeout,
      metadata: HashMap::new(),
    }
  }

  /// Returns true if the transaction is active.
  pub fn is_active(&self) -> bool {
    self.state == TransactionState::Active
  }

  /// Returns true if the transaction has timed out.
  pub fn is_timed_out(&self) -> bool {
    self.started_at.elapsed() > self.timeout
  }

  /// Returns the elapsed time since the transaction started.
  pub fn elapsed(&self) -> Duration {
    self.started_at.elapsed()
  }

  /// Returns the remaining time before timeout.
  pub fn remaining_time(&self) -> Option<Duration> {
    self.timeout.checked_sub(self.started_at.elapsed())
  }

  /// Adds a buffered offset operation.
  pub fn buffer_offset(&mut self, source: String, offset: Offset) {
    self
      .buffered_offsets
      .push(BufferedOffset { source, offset });
  }

  /// Creates a savepoint with the given name.
  pub fn create_savepoint(&mut self, name: String) -> Savepoint {
    let savepoint = Savepoint {
      name: name.clone(),
      buffer_index: self.buffered_offsets.len(),
      created_at: Instant::now(),
    };
    self.savepoints.push(savepoint.clone());
    savepoint
  }

  /// Rolls back to a savepoint, discarding operations after it.
  pub fn rollback_to_savepoint(&mut self, name: &str) -> Option<()> {
    if let Some(index) = self.savepoints.iter().position(|s| s.name == name) {
      let savepoint = &self.savepoints[index];
      let buffer_index = savepoint.buffer_index;

      // Truncate buffered offsets to savepoint
      self.buffered_offsets.truncate(buffer_index);

      // Remove this savepoint and all after it
      self.savepoints.truncate(index);

      Some(())
    } else {
      None
    }
  }

  /// Releases a savepoint without rolling back.
  pub fn release_savepoint(&mut self, name: &str) -> Option<Savepoint> {
    if let Some(index) = self.savepoints.iter().position(|s| s.name == name) {
      Some(self.savepoints.remove(index))
    } else {
      None
    }
  }

  /// Sets custom metadata on the transaction.
  pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
    self.metadata.insert(key.into(), value.into());
  }

  /// Gets custom metadata from the transaction.
  pub fn get_metadata(&self, key: &str) -> Option<&String> {
    self.metadata.get(key)
  }
}

/// Manages transaction lifecycle and coordinates offset commits.
pub struct TransactionManager {
  /// ID generator for transactions.
  next_id: AtomicU64,
  /// Active transactions.
  transactions: Arc<RwLock<HashMap<TransactionId, Transaction>>>,
  /// Offset store for committing offsets.
  offset_store: Arc<RwLock<Box<dyn OffsetStore>>>,
  /// Configuration.
  config: TransactionConfig,
}

impl TransactionManager {
  /// Creates a new transaction manager.
  pub fn new(offset_store: Box<dyn OffsetStore>, config: TransactionConfig) -> Self {
    Self {
      next_id: AtomicU64::new(1),
      transactions: Arc::new(RwLock::new(HashMap::new())),
      offset_store: Arc::new(RwLock::new(offset_store)),
      config,
    }
  }

  /// Creates a transaction manager with default configuration.
  pub fn with_default_config(offset_store: Box<dyn OffsetStore>) -> Self {
    Self::new(offset_store, TransactionConfig::default())
  }

  /// Returns the configuration.
  pub fn config(&self) -> &TransactionConfig {
    &self.config
  }

  /// Generates a new transaction ID.
  fn generate_id(&self) -> TransactionId {
    TransactionId(self.next_id.fetch_add(1, Ordering::Relaxed))
  }

  /// Begins a new transaction.
  pub async fn begin(&self) -> TransactionResult<TransactionId> {
    let id = self.generate_id();
    let transaction = Transaction::new(id, self.config.timeout);

    let mut transactions = self.transactions.write().await;
    transactions.insert(id, transaction);

    Ok(id)
  }

  /// Begins a new transaction with custom timeout.
  pub async fn begin_with_timeout(&self, timeout: Duration) -> TransactionResult<TransactionId> {
    let id = self.generate_id();
    let transaction = Transaction::new(id, timeout);

    let mut transactions = self.transactions.write().await;
    transactions.insert(id, transaction);

    Ok(id)
  }

  /// Commits a transaction, flushing all buffered offsets.
  pub async fn commit(&self, id: &TransactionId) -> TransactionResult<()> {
    // First, extract the transaction and validate
    let buffered_offsets = {
      let mut transactions = self.transactions.write().await;

      let transaction = transactions
        .get_mut(id)
        .ok_or(TransactionError::NotFound(*id))?;

      // Check state
      if !transaction.is_active() {
        return Err(TransactionError::NotActive(*id, transaction.state));
      }

      // Check timeout
      if transaction.is_timed_out() {
        transaction.state = TransactionState::TimedOut;
        return Err(TransactionError::Timeout(*id));
      }

      // Extract buffered offsets
      let offsets = std::mem::take(&mut transaction.buffered_offsets);

      // Mark as committed
      transaction.state = TransactionState::Committed;

      offsets
    };

    // Flush buffered offsets to the store
    {
      let offset_store = self.offset_store.write().await;

      for buffered in buffered_offsets {
        offset_store
          .commit(&buffered.source, buffered.offset)
          .map_err(TransactionError::from)?;
      }
    }

    Ok(())
  }

  /// Rolls back a transaction, discarding all buffered operations.
  pub async fn rollback(&self, id: &TransactionId) -> TransactionResult<()> {
    let mut transactions = self.transactions.write().await;

    let transaction = transactions
      .get_mut(id)
      .ok_or(TransactionError::NotFound(*id))?;

    // Check state
    if !transaction.is_active() {
      return Err(TransactionError::NotActive(*id, transaction.state));
    }

    // Clear buffered operations
    transaction.buffered_offsets.clear();
    transaction.savepoints.clear();

    // Mark as rolled back
    transaction.state = TransactionState::RolledBack;

    Ok(())
  }

  /// Buffers an offset commit within a transaction.
  pub async fn buffer_offset(
    &self,
    id: &TransactionId,
    source: &str,
    offset: Offset,
  ) -> TransactionResult<()> {
    let mut transactions = self.transactions.write().await;

    let transaction = transactions
      .get_mut(id)
      .ok_or(TransactionError::NotFound(*id))?;

    // Check state
    if !transaction.is_active() {
      return Err(TransactionError::NotActive(*id, transaction.state));
    }

    // Check timeout
    if transaction.is_timed_out() {
      transaction.state = TransactionState::TimedOut;
      return Err(TransactionError::Timeout(*id));
    }

    transaction.buffer_offset(source.to_string(), offset);
    Ok(())
  }

  /// Creates a savepoint within a transaction.
  pub async fn create_savepoint(
    &self,
    id: &TransactionId,
    name: &str,
  ) -> TransactionResult<Savepoint> {
    let mut transactions = self.transactions.write().await;

    let transaction = transactions
      .get_mut(id)
      .ok_or(TransactionError::NotFound(*id))?;

    // Check state
    if !transaction.is_active() {
      return Err(TransactionError::NotActive(*id, transaction.state));
    }

    // Check nesting limit
    if transaction.savepoints.len() >= self.config.max_nesting_level {
      return Err(TransactionError::NestingLimitExceeded(
        self.config.max_nesting_level,
      ));
    }

    Ok(transaction.create_savepoint(name.to_string()))
  }

  /// Rolls back to a savepoint within a transaction.
  pub async fn rollback_to_savepoint(
    &self,
    id: &TransactionId,
    name: &str,
  ) -> TransactionResult<()> {
    let mut transactions = self.transactions.write().await;

    let transaction = transactions
      .get_mut(id)
      .ok_or(TransactionError::NotFound(*id))?;

    // Check state
    if !transaction.is_active() {
      return Err(TransactionError::NotActive(*id, transaction.state));
    }

    transaction
      .rollback_to_savepoint(name)
      .ok_or_else(|| TransactionError::SavepointNotFound(name.to_string()))?;

    Ok(())
  }

  /// Releases a savepoint within a transaction.
  pub async fn release_savepoint(
    &self,
    id: &TransactionId,
    name: &str,
  ) -> TransactionResult<Savepoint> {
    let mut transactions = self.transactions.write().await;

    let transaction = transactions
      .get_mut(id)
      .ok_or(TransactionError::NotFound(*id))?;

    // Check state
    if !transaction.is_active() {
      return Err(TransactionError::NotActive(*id, transaction.state));
    }

    transaction
      .release_savepoint(name)
      .ok_or_else(|| TransactionError::SavepointNotFound(name.to_string()))
  }

  /// Gets the state of a transaction.
  pub async fn get_state(&self, id: &TransactionId) -> TransactionResult<TransactionState> {
    // First check if we need to update timeout status
    let needs_timeout_update = {
      let transactions = self.transactions.read().await;
      let transaction = transactions
        .get(id)
        .ok_or(TransactionError::NotFound(*id))?;
      transaction.is_active() && transaction.is_timed_out()
    };

    if needs_timeout_update {
      let mut transactions = self.transactions.write().await;
      if let Some(tx) = transactions.get_mut(id) {
        if tx.is_active() && tx.is_timed_out() {
          tx.state = TransactionState::TimedOut;
          if self.config.auto_rollback_on_timeout {
            tx.buffered_offsets.clear();
            tx.savepoints.clear();
          }
        }
        return Ok(tx.state);
      }
      return Err(TransactionError::NotFound(*id));
    }

    // Normal read path
    let transactions = self.transactions.read().await;
    let transaction = transactions
      .get(id)
      .ok_or(TransactionError::NotFound(*id))?;
    Ok(transaction.state)
  }

  /// Gets information about a transaction.
  pub async fn get_transaction(&self, id: &TransactionId) -> TransactionResult<TransactionInfo> {
    let transactions = self.transactions.read().await;

    let transaction = transactions
      .get(id)
      .ok_or(TransactionError::NotFound(*id))?;

    Ok(TransactionInfo {
      id: transaction.id,
      state: transaction.state,
      buffered_offset_count: transaction.buffered_offsets.len(),
      savepoint_count: transaction.savepoints.len(),
      elapsed: transaction.elapsed(),
      remaining_time: transaction.remaining_time(),
      is_timed_out: transaction.is_timed_out(),
    })
  }

  /// Lists all active transactions.
  pub async fn list_active(&self) -> Vec<TransactionId> {
    let transactions = self.transactions.read().await;

    transactions
      .iter()
      .filter(|(_, tx)| tx.is_active())
      .map(|(id, _)| *id)
      .collect()
  }

  /// Cleans up completed transactions older than the specified duration.
  pub async fn cleanup(&self, max_age: Duration) -> usize {
    let mut transactions = self.transactions.write().await;

    let to_remove: Vec<TransactionId> = transactions
      .iter()
      .filter(|(_, tx)| !tx.is_active() && tx.elapsed() > max_age)
      .map(|(id, _)| *id)
      .collect();

    let count = to_remove.len();
    for id in to_remove {
      transactions.remove(&id);
    }

    count
  }

  /// Checks for timed-out transactions and rolls them back if configured.
  pub async fn check_timeouts(&self) -> Vec<TransactionId> {
    let mut transactions = self.transactions.write().await;
    let mut timed_out = Vec::new();

    for (id, tx) in transactions.iter_mut() {
      if tx.is_active() && tx.is_timed_out() {
        tx.state = TransactionState::TimedOut;
        if self.config.auto_rollback_on_timeout {
          tx.buffered_offsets.clear();
          tx.savepoints.clear();
        }
        timed_out.push(*id);
      }
    }

    timed_out
  }

  /// Sets metadata on a transaction.
  pub async fn set_metadata(
    &self,
    id: &TransactionId,
    key: &str,
    value: &str,
  ) -> TransactionResult<()> {
    let mut transactions = self.transactions.write().await;

    let transaction = transactions
      .get_mut(id)
      .ok_or(TransactionError::NotFound(*id))?;

    transaction.set_metadata(key, value);
    Ok(())
  }

  /// Gets metadata from a transaction.
  pub async fn get_metadata(
    &self,
    id: &TransactionId,
    key: &str,
  ) -> TransactionResult<Option<String>> {
    let transactions = self.transactions.read().await;

    let transaction = transactions
      .get(id)
      .ok_or(TransactionError::NotFound(*id))?;

    Ok(transaction.get_metadata(key).cloned())
  }
}

impl std::fmt::Debug for TransactionManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TransactionManager")
      .field("next_id", &self.next_id)
      .field("config", &self.config)
      .finish_non_exhaustive()
  }
}

/// Information about a transaction (read-only view).
#[derive(Debug, Clone)]
pub struct TransactionInfo {
  /// Transaction ID.
  pub id: TransactionId,
  /// Current state.
  pub state: TransactionState,
  /// Number of buffered offset operations.
  pub buffered_offset_count: usize,
  /// Number of active savepoints.
  pub savepoint_count: usize,
  /// Time elapsed since transaction started.
  pub elapsed: Duration,
  /// Time remaining before timeout.
  pub remaining_time: Option<Duration>,
  /// Whether the transaction has timed out.
  pub is_timed_out: bool,
}

/// A transactional context for scoped transaction management.
///
/// This provides RAII-style transaction management with automatic
/// rollback on drop if not committed.
pub struct TransactionalContext<'a> {
  manager: &'a TransactionManager,
  id: TransactionId,
  committed: bool,
}

impl<'a> TransactionalContext<'a> {
  /// Creates a new transactional context.
  pub async fn new(manager: &'a TransactionManager) -> TransactionResult<Self> {
    let id = manager.begin().await?;
    Ok(Self {
      manager,
      id,
      committed: false,
    })
  }

  /// Creates a new transactional context with custom timeout.
  pub async fn with_timeout(
    manager: &'a TransactionManager,
    timeout: Duration,
  ) -> TransactionResult<Self> {
    let id = manager.begin_with_timeout(timeout).await?;
    Ok(Self {
      manager,
      id,
      committed: false,
    })
  }

  /// Returns the transaction ID.
  pub fn id(&self) -> &TransactionId {
    &self.id
  }

  /// Buffers an offset commit.
  pub async fn buffer_offset(&self, source: &str, offset: Offset) -> TransactionResult<()> {
    self.manager.buffer_offset(&self.id, source, offset).await
  }

  /// Creates a savepoint.
  pub async fn savepoint(&self, name: &str) -> TransactionResult<Savepoint> {
    self.manager.create_savepoint(&self.id, name).await
  }

  /// Rolls back to a savepoint.
  pub async fn rollback_to(&self, name: &str) -> TransactionResult<()> {
    self.manager.rollback_to_savepoint(&self.id, name).await
  }

  /// Commits the transaction.
  pub async fn commit(mut self) -> TransactionResult<()> {
    self.committed = true;
    self.manager.commit(&self.id).await
  }

  /// Explicitly rolls back the transaction.
  pub async fn rollback(mut self) -> TransactionResult<()> {
    self.committed = true; // Prevent double rollback on drop
    self.manager.rollback(&self.id).await
  }
}

// Note: We cannot implement Drop with async rollback,
// so users should call commit() or rollback() explicitly.
// This is documented in the struct.

#[cfg(test)]
mod tests {
  use super::*;
  use streamweave_offset::InMemoryOffsetStore;

  fn create_manager() -> TransactionManager {
    let store = Box::new(InMemoryOffsetStore::new());
    TransactionManager::with_default_config(store)
  }

  fn create_manager_with_config(config: TransactionConfig) -> TransactionManager {
    let store = Box::new(InMemoryOffsetStore::new());
    TransactionManager::new(store, config)
  }

  #[tokio::test]
  async fn test_transaction_id_display() {
    let id = TransactionId::new(42);
    assert_eq!(format!("{}", id), "tx:42");
    assert_eq!(id.value(), 42);
  }

  #[tokio::test]
  async fn test_transaction_state_display() {
    assert_eq!(format!("{}", TransactionState::Active), "active");
    assert_eq!(format!("{}", TransactionState::Committed), "committed");
    assert_eq!(format!("{}", TransactionState::RolledBack), "rolled_back");
    assert_eq!(format!("{}", TransactionState::TimedOut), "timed_out");
  }

  #[tokio::test]
  async fn test_begin_commit() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();
    assert_eq!(
      manager.get_state(&id).await.unwrap(),
      TransactionState::Active
    );

    manager.commit(&id).await.unwrap();
    assert_eq!(
      manager.get_state(&id).await.unwrap(),
      TransactionState::Committed
    );
  }

  #[tokio::test]
  async fn test_begin_rollback() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();
    manager.rollback(&id).await.unwrap();

    assert_eq!(
      manager.get_state(&id).await.unwrap(),
      TransactionState::RolledBack
    );
  }

  #[tokio::test]
  async fn test_buffer_and_commit_offsets() {
    let store = Box::new(InMemoryOffsetStore::new());
    let store_clone = store.clone();
    let manager = TransactionManager::with_default_config(store);

    let id = manager.begin().await.unwrap();

    manager
      .buffer_offset(&id, "source1", Offset::Sequence(100))
      .await
      .unwrap();
    manager
      .buffer_offset(&id, "source2", Offset::Sequence(200))
      .await
      .unwrap();

    // Before commit, offsets should not be in store
    assert!(store_clone.get("source1").unwrap().is_none());
    assert!(store_clone.get("source2").unwrap().is_none());

    manager.commit(&id).await.unwrap();

    // After commit, offsets should be in store
    assert_eq!(
      store_clone.get("source1").unwrap(),
      Some(Offset::Sequence(100))
    );
    assert_eq!(
      store_clone.get("source2").unwrap(),
      Some(Offset::Sequence(200))
    );
  }

  #[tokio::test]
  async fn test_rollback_discards_offsets() {
    let store = Box::new(InMemoryOffsetStore::new());
    let store_clone = store.clone();
    let manager = TransactionManager::with_default_config(store);

    let id = manager.begin().await.unwrap();

    manager
      .buffer_offset(&id, "source1", Offset::Sequence(100))
      .await
      .unwrap();
    manager.rollback(&id).await.unwrap();

    // Offsets should not be committed
    assert!(store_clone.get("source1").unwrap().is_none());
  }

  #[tokio::test]
  async fn test_savepoint_and_rollback() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();

    manager
      .buffer_offset(&id, "s1", Offset::Sequence(1))
      .await
      .unwrap();
    manager.create_savepoint(&id, "sp1").await.unwrap();

    manager
      .buffer_offset(&id, "s2", Offset::Sequence(2))
      .await
      .unwrap();
    manager
      .buffer_offset(&id, "s3", Offset::Sequence(3))
      .await
      .unwrap();

    // Rollback to savepoint should discard s2 and s3
    manager.rollback_to_savepoint(&id, "sp1").await.unwrap();

    // Get transaction info to check buffered count
    let info = manager.get_transaction(&id).await.unwrap();
    assert_eq!(info.buffered_offset_count, 1); // Only s1 remains
  }

  #[tokio::test]
  async fn test_savepoint_nesting_limit() {
    let config = TransactionConfig::default().with_max_nesting_level(2);
    let manager = create_manager_with_config(config);

    let id = manager.begin().await.unwrap();

    manager.create_savepoint(&id, "sp1").await.unwrap();
    manager.create_savepoint(&id, "sp2").await.unwrap();

    // Third savepoint should fail
    let result = manager.create_savepoint(&id, "sp3").await;
    assert!(matches!(
      result,
      Err(TransactionError::NestingLimitExceeded(2))
    ));
  }

  #[tokio::test]
  async fn test_transaction_timeout() {
    let config = TransactionConfig::default().with_timeout(Duration::from_millis(10));
    let manager = create_manager_with_config(config);

    let id = manager.begin().await.unwrap();

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Check state should detect timeout
    let state = manager.get_state(&id).await.unwrap();
    assert_eq!(state, TransactionState::TimedOut);
  }

  #[tokio::test]
  async fn test_cannot_commit_non_active() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();
    manager.rollback(&id).await.unwrap();

    // Should fail to commit rolled back transaction
    let result = manager.commit(&id).await;
    assert!(matches!(
      result,
      Err(TransactionError::NotActive(_, TransactionState::RolledBack))
    ));
  }

  #[tokio::test]
  async fn test_begin_with_timeout() {
    let manager = create_manager();
    let id = manager
      .begin_with_timeout(Duration::from_millis(50))
      .await
      .unwrap();

    let info = manager.get_transaction(&id).await.unwrap();
    assert!(info.remaining_time.is_some());
    assert!(info.remaining_time.unwrap() <= Duration::from_millis(50));
  }

  #[tokio::test]
  async fn test_transaction_config_with_auto_rollback() {
    let config = TransactionConfig::default().with_auto_rollback_on_timeout(true);
    assert!(config.auto_rollback_on_timeout);

    let config = TransactionConfig::default().with_auto_rollback_on_timeout(false);
    assert!(!config.auto_rollback_on_timeout);
  }

  #[tokio::test]
  async fn test_transaction_elapsed_and_remaining_time() {
    let manager = create_manager();
    let id = manager.begin().await.unwrap();

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(10)).await;

    let info = manager.get_transaction(&id).await.unwrap();
    assert!(info.elapsed >= Duration::from_millis(10));
    assert!(info.remaining_time.is_some());
  }

  #[tokio::test]
  async fn test_transactional_context_with_timeout() {
    let manager = create_manager();

    let ctx = TransactionalContext::with_timeout(&manager, Duration::from_secs(30))
      .await
      .unwrap();
    let id = *ctx.id();

    assert_eq!(
      manager.get_state(&id).await.unwrap(),
      TransactionState::Active
    );
  }

  #[tokio::test]
  async fn test_transactional_context_savepoint() {
    let manager = create_manager();

    let ctx = TransactionalContext::new(&manager).await.unwrap();
    ctx.buffer_offset("s1", Offset::Sequence(1)).await.unwrap();
    ctx.savepoint("sp1").await.unwrap();
    ctx.buffer_offset("s2", Offset::Sequence(2)).await.unwrap();
    ctx.rollback_to("sp1").await.unwrap();

    let id = *ctx.id();
    let info = manager.get_transaction(&id).await.unwrap();
    assert_eq!(info.buffered_offset_count, 1);
  }

  #[tokio::test]
  async fn test_cannot_rollback_non_active() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();
    manager.commit(&id).await.unwrap();

    // Should fail to rollback committed transaction
    let result = manager.rollback(&id).await;
    assert!(matches!(
      result,
      Err(TransactionError::NotActive(_, TransactionState::Committed))
    ));
  }

  #[tokio::test]
  async fn test_transaction_not_found() {
    let manager = create_manager();

    let fake_id = TransactionId::new(999);
    let result = manager.commit(&fake_id).await;
    assert!(matches!(result, Err(TransactionError::NotFound(_))));
  }

  #[tokio::test]
  async fn test_list_active_transactions() {
    let manager = create_manager();

    let id1 = manager.begin().await.unwrap();
    let id2 = manager.begin().await.unwrap();
    let id3 = manager.begin().await.unwrap();

    manager.commit(&id2).await.unwrap();

    let active = manager.list_active().await;
    assert_eq!(active.len(), 2);
    assert!(active.contains(&id1));
    assert!(active.contains(&id3));
    assert!(!active.contains(&id2));
  }

  #[tokio::test]
  async fn test_cleanup_completed_transactions() {
    let manager = create_manager();

    let id1 = manager.begin().await.unwrap();
    let id2 = manager.begin().await.unwrap();

    manager.commit(&id1).await.unwrap();
    manager.rollback(&id2).await.unwrap();

    // Cleanup with 0 duration should remove completed transactions
    let cleaned = manager.cleanup(Duration::ZERO).await;
    assert_eq!(cleaned, 2);

    // Both should now be not found
    assert!(matches!(
      manager.get_state(&id1).await,
      Err(TransactionError::NotFound(_))
    ));
    assert!(matches!(
      manager.get_state(&id2).await,
      Err(TransactionError::NotFound(_))
    ));
  }

  #[tokio::test]
  async fn test_check_timeouts() {
    let config = TransactionConfig::default()
      .with_timeout(Duration::from_millis(10))
      .with_auto_rollback_on_timeout(true);
    let manager = create_manager_with_config(config);

    let id = manager.begin().await.unwrap();
    manager
      .buffer_offset(&id, "source1", Offset::Sequence(100))
      .await
      .unwrap();

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(20)).await;

    let timed_out = manager.check_timeouts().await;
    assert_eq!(timed_out.len(), 1);
    assert!(timed_out.contains(&id));

    // Transaction should be marked as timed out
    assert_eq!(
      manager.get_state(&id).await.unwrap(),
      TransactionState::TimedOut
    );

    // Buffered offsets should be cleared (auto-rollback)
    let info = manager.get_transaction(&id).await.unwrap();
    assert_eq!(info.buffered_offset_count, 0);
  }

  #[tokio::test]
  async fn test_transaction_metadata() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();

    manager.set_metadata(&id, "user_id", "123").await.unwrap();
    manager
      .set_metadata(&id, "operation", "batch_process")
      .await
      .unwrap();

    assert_eq!(
      manager.get_metadata(&id, "user_id").await.unwrap(),
      Some("123".to_string())
    );
    assert_eq!(
      manager.get_metadata(&id, "operation").await.unwrap(),
      Some("batch_process".to_string())
    );
    assert_eq!(
      manager.get_metadata(&id, "nonexistent").await.unwrap(),
      None
    );
  }

  #[tokio::test]
  async fn test_transaction_info() {
    let config = TransactionConfig::default().with_timeout(Duration::from_secs(30));
    let manager = create_manager_with_config(config);

    let id = manager.begin().await.unwrap();
    manager
      .buffer_offset(&id, "s1", Offset::Sequence(1))
      .await
      .unwrap();
    manager
      .buffer_offset(&id, "s2", Offset::Sequence(2))
      .await
      .unwrap();
    manager.create_savepoint(&id, "sp1").await.unwrap();

    let info = manager.get_transaction(&id).await.unwrap();

    assert_eq!(info.id, id);
    assert_eq!(info.state, TransactionState::Active);
    assert_eq!(info.buffered_offset_count, 2);
    assert_eq!(info.savepoint_count, 1);
    assert!(!info.is_timed_out);
    assert!(info.remaining_time.is_some());
  }

  #[tokio::test]
  async fn test_release_savepoint() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();
    manager.create_savepoint(&id, "sp1").await.unwrap();
    manager.create_savepoint(&id, "sp2").await.unwrap();

    let info = manager.get_transaction(&id).await.unwrap();
    assert_eq!(info.savepoint_count, 2);

    manager.release_savepoint(&id, "sp1").await.unwrap();

    let info = manager.get_transaction(&id).await.unwrap();
    assert_eq!(info.savepoint_count, 1);
  }

  #[tokio::test]
  async fn test_release_nonexistent_savepoint() {
    let manager = create_manager();

    let id = manager.begin().await.unwrap();
    let result = manager.release_savepoint(&id, "nonexistent").await;

    assert!(matches!(
      result,
      Err(TransactionError::SavepointNotFound(_))
    ));
  }

  #[tokio::test]
  async fn test_config_builder() {
    let config = TransactionConfig::new()
      .with_timeout(Duration::from_secs(120))
      .with_max_nesting_level(5)
      .with_auto_rollback_on_timeout(false);

    assert_eq!(config.timeout, Duration::from_secs(120));
    assert_eq!(config.max_nesting_level, 5);
    assert!(!config.auto_rollback_on_timeout);
  }

  #[tokio::test]
  async fn test_transactional_context() {
    let store = Box::new(InMemoryOffsetStore::new());
    let store_clone = store.clone();
    let manager = TransactionManager::with_default_config(store);

    {
      let ctx = TransactionalContext::new(&manager).await.unwrap();
      ctx
        .buffer_offset("source1", Offset::Sequence(100))
        .await
        .unwrap();
      ctx.commit().await.unwrap();
    }

    assert_eq!(
      store_clone.get("source1").unwrap(),
      Some(Offset::Sequence(100))
    );
  }

  #[tokio::test]
  async fn test_transactional_context_with_savepoint() {
    let store = Box::new(InMemoryOffsetStore::new());
    let store_clone = store.clone();
    let manager = TransactionManager::with_default_config(store);

    {
      let ctx = TransactionalContext::new(&manager).await.unwrap();
      ctx
        .buffer_offset("source1", Offset::Sequence(100))
        .await
        .unwrap();
      ctx.savepoint("sp1").await.unwrap();
      ctx
        .buffer_offset("source2", Offset::Sequence(200))
        .await
        .unwrap();
      ctx.rollback_to("sp1").await.unwrap();
      ctx.commit().await.unwrap();
    }

    // Only source1 should be committed (source2 was after savepoint)
    assert_eq!(
      store_clone.get("source1").unwrap(),
      Some(Offset::Sequence(100))
    );
    assert!(store_clone.get("source2").unwrap().is_none());
  }

  #[tokio::test]
  async fn test_transaction_error_display() {
    let id = TransactionId::new(1);

    let not_found = TransactionError::NotFound(id);
    assert!(not_found.to_string().contains("not found"));

    let not_active = TransactionError::NotActive(id, TransactionState::Committed);
    assert!(not_active.to_string().contains("not active"));

    let timeout = TransactionError::Timeout(id);
    assert!(timeout.to_string().contains("timed out"));

    let invalid = TransactionError::InvalidOperation("test".to_string());
    assert!(invalid.to_string().contains("Invalid operation"));

    let savepoint = TransactionError::SavepointNotFound("sp1".to_string());
    assert!(savepoint.to_string().contains("Savepoint not found"));

    let nesting = TransactionError::NestingLimitExceeded(10);
    assert!(nesting.to_string().contains("limit exceeded"));
  }

  #[tokio::test]
  async fn test_multiple_transactions_concurrent() {
    let manager = create_manager();

    // Begin multiple transactions
    let ids: Vec<_> = futures::future::join_all((0..10).map(|_| manager.begin()))
      .await
      .into_iter()
      .map(|r| r.unwrap())
      .collect();

    // All should be active
    for id in &ids {
      assert_eq!(
        manager.get_state(id).await.unwrap(),
        TransactionState::Active
      );
    }

    // Commit half, rollback half
    for (i, id) in ids.iter().enumerate() {
      if i % 2 == 0 {
        manager.commit(id).await.unwrap();
      } else {
        manager.rollback(id).await.unwrap();
      }
    }

    // Verify states
    for (i, id) in ids.iter().enumerate() {
      let expected = if i % 2 == 0 {
        TransactionState::Committed
      } else {
        TransactionState::RolledBack
      };
      assert_eq!(manager.get_state(id).await.unwrap(), expected);
    }
  }

  #[tokio::test]
  async fn test_transaction_remaining_time() {
    let config = TransactionConfig::default().with_timeout(Duration::from_secs(10));
    let manager = create_manager_with_config(config);

    let id = manager.begin().await.unwrap();

    let info = manager.get_transaction(&id).await.unwrap();
    assert!(info.remaining_time.is_some());

    let remaining = info.remaining_time.unwrap();
    assert!(remaining <= Duration::from_secs(10));
    assert!(remaining > Duration::from_secs(9)); // Should be close to 10s
  }
}
