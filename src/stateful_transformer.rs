//! Stateful transformer trait for stream processing with persistent state.
//!
//! This module provides the [`StatefulTransformer`] trait which extends the base
//! [`Transformer`] trait with state management capabilities.
//! State is thread-safe and persists across stream items, enabling use cases like:
//!
//! - Running aggregations (sum, average, count)
//! - Session management
//! - Pattern detection across items
//! - Stateful windowing operations
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::stateful_transformer::{StatefulTransformer, InMemoryStateStore};
//!
//! struct RunningSumTransformer {
//!     state: InMemoryStateStore<i64>,
//!     config: TransformerConfig<i32>,
//! }
//!
//! impl StatefulTransformer for RunningSumTransformer {
//!     type State = i64;
//!     type Store = InMemoryStateStore<i64>;
//!
//!     fn state_store(&self) -> &Self::Store {
//!         &self.state
//!     }
//!
//!     fn state_store_mut(&mut self) -> &mut Self::Store {
//!         &mut self.state
//!     }
//! }
//! ```

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::error::ErrorStrategy;
use crate::{Transformer, TransformerConfig};

/// Error type for state operations.
#[derive(Debug, Clone)]
pub enum StateError {
  /// State is not initialized
  NotInitialized,
  /// Lock acquisition failed (poisoned)
  LockPoisoned,
  /// State update failed
  UpdateFailed(String),
  /// Serialization failed
  SerializationFailed(String),
  /// Deserialization failed
  DeserializationFailed(String),
}

impl std::fmt::Display for StateError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StateError::NotInitialized => write!(f, "State is not initialized"),
      StateError::LockPoisoned => write!(f, "State lock is poisoned"),
      StateError::UpdateFailed(msg) => write!(f, "State update failed: {}", msg),
      StateError::SerializationFailed(msg) => write!(f, "State serialization failed: {}", msg),
      StateError::DeserializationFailed(msg) => {
        write!(f, "State deserialization failed: {}", msg)
      }
    }
  }
}

impl std::error::Error for StateError {}

/// Result type for state operations.
pub type StateResult<T> = Result<T, StateError>;

/// Trait for state storage backends.
///
/// This trait abstracts the storage mechanism for transformer state,
/// allowing for different implementations (in-memory, persistent, etc.).
///
/// The trait uses `Box<dyn FnOnce>` for the update function to maintain
/// dyn compatibility while still supporting closures.
pub trait StateStore<S>: Send + Sync
where
  S: Clone + Send + Sync,
{
  /// Get a read-only reference to the current state.
  ///
  /// Returns `None` if the state has not been initialized.
  fn get(&self) -> StateResult<Option<S>>;

  /// Set the state to a new value.
  fn set(&self, state: S) -> StateResult<()>;

  /// Update the state using a boxed function.
  ///
  /// The function receives the current state (if any) and returns the new state.
  fn update_with(&self, f: Box<dyn FnOnce(Option<S>) -> S + Send>) -> StateResult<S>;

  /// Reset the state to its initial value or clear it.
  fn reset(&self) -> StateResult<()>;

  /// Check if the state has been initialized.
  fn is_initialized(&self) -> bool;

  /// Get the initial state value if one was provided.
  fn initial_state(&self) -> Option<S>;
}

/// Extension trait for convenient state updates with closures.
///
/// This trait provides a more ergonomic `update` method that accepts
/// any closure, boxing it internally.
pub trait StateStoreExt<S>: StateStore<S>
where
  S: Clone + Send + Sync + 'static,
{
  /// Update the state using a closure.
  ///
  /// This is a convenience method that boxes the closure internally.
  fn update<F>(&self, f: F) -> StateResult<S>
  where
    F: FnOnce(Option<S>) -> S + Send + 'static,
  {
    self.update_with(Box::new(f))
  }
}

// Blanket implementation for all StateStore types
impl<S, T> StateStoreExt<S> for T
where
  S: Clone + Send + Sync + 'static,
  T: StateStore<S>,
{
}

/// Extension trait for state checkpointing (serialization/deserialization).
///
/// This trait provides methods for serializing state to bytes and
/// restoring state from bytes, enabling checkpointing and persistence.
///
/// # Example
///
/// ```rust
/// use crate::stateful_transformer::{InMemoryStateStore, StateStore, StateCheckpoint};
///
/// let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
///
/// // Serialize state to bytes
/// let checkpoint = store.serialize_state().unwrap();
///
/// // Restore to a new store
/// let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
/// store2.deserialize_and_set_state(&checkpoint).unwrap();
///
/// assert_eq!(store2.get().unwrap(), Some(42));
/// ```
pub trait StateCheckpoint<S>: StateStore<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + Default,
{
  /// Serialize the current state to a byte vector.
  ///
  /// This is used for checkpointing and persistence.
  /// Returns an empty vector if no state is set.
  fn serialize_state(&self) -> StateResult<Vec<u8>> {
    self
      .get()?
      .map(|s| serde_json::to_vec(&s).map_err(|e| StateError::SerializationFailed(e.to_string())))
      .unwrap_or(Ok(Vec::new()))
  }

  /// Deserialize state from a byte vector and set it.
  ///
  /// This is used for restoring state from checkpoints.
  /// If the data is empty, sets the state to the default value.
  fn deserialize_and_set_state(&self, data: &[u8]) -> StateResult<()> {
    if data.is_empty() {
      self.set(S::default())
    } else {
      let state: S = serde_json::from_slice(data)
        .map_err(|e| StateError::DeserializationFailed(e.to_string()))?;
      self.set(state)
    }
  }
}

// Blanket implementation for all StateStore types with serialization support
impl<S, T> StateCheckpoint<S> for T
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + Default,
  T: StateStore<S>,
{
}

/// In-memory state store using `Arc<RwLock<S>>` for thread-safe access.
///
/// This is the default state store implementation that keeps state in memory.
/// It is suitable for single-process, non-distributed use cases.
///
/// # Thread Safety
///
/// All operations are thread-safe and can be called concurrently from
/// multiple threads without external synchronization.
///
/// # Example
///
/// ```rust
/// use crate::stateful_transformer::{InMemoryStateStore, StateStore, StateStoreExt};
///
/// let store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);
/// store.set(42).unwrap();
/// assert_eq!(store.get().unwrap(), Some(42));
///
/// // Use update with a closure
/// store.update(|current| current.unwrap_or(0) + 10).unwrap();
/// assert_eq!(store.get().unwrap(), Some(52));
/// ```
#[derive(Debug)]
pub struct InMemoryStateStore<S>
where
  S: Clone + Send + Sync,
{
  state: Arc<RwLock<Option<S>>>,
  initial: Option<S>,
}

impl<S> InMemoryStateStore<S>
where
  S: Clone + Send + Sync,
{
  /// Create a new in-memory state store with an initial value.
  pub fn new(initial: S) -> Self {
    Self {
      state: Arc::new(RwLock::new(Some(initial.clone()))),
      initial: Some(initial),
    }
  }

  /// Create a new in-memory state store without an initial value.
  pub fn empty() -> Self {
    Self {
      state: Arc::new(RwLock::new(None)),
      initial: None,
    }
  }

  /// Create a new in-memory state store with an optional initial value.
  pub fn with_optional_initial(initial: Option<S>) -> Self {
    Self {
      state: Arc::new(RwLock::new(initial.clone())),
      initial,
    }
  }

  /// Get a read guard for the state.
  pub fn read(&self) -> StateResult<RwLockReadGuard<'_, Option<S>>> {
    self.state.read().map_err(|_| StateError::LockPoisoned)
  }

  /// Get a write guard for the state.
  pub fn write(&self) -> StateResult<RwLockWriteGuard<'_, Option<S>>> {
    self.state.write().map_err(|_| StateError::LockPoisoned)
  }
}

impl<S> Clone for InMemoryStateStore<S>
where
  S: Clone + Send + Sync,
{
  fn clone(&self) -> Self {
    // Clone creates a new store with the same current state
    let current = self.state.read().ok().and_then(|guard| guard.clone());
    Self {
      state: Arc::new(RwLock::new(current)),
      initial: self.initial.clone(),
    }
  }
}

impl<S> Default for InMemoryStateStore<S>
where
  S: Clone + Send + Sync + Default,
{
  fn default() -> Self {
    Self::new(S::default())
  }
}

impl<S> StateStore<S> for InMemoryStateStore<S>
where
  S: Clone + Send + Sync,
{
  fn get(&self) -> StateResult<Option<S>> {
    let guard = self.state.read().map_err(|_| StateError::LockPoisoned)?;
    Ok(guard.clone())
  }

  fn set(&self, state: S) -> StateResult<()> {
    let mut guard = self.state.write().map_err(|_| StateError::LockPoisoned)?;
    *guard = Some(state);
    Ok(())
  }

  fn update_with(&self, f: Box<dyn FnOnce(Option<S>) -> S + Send>) -> StateResult<S> {
    let mut guard = self.state.write().map_err(|_| StateError::LockPoisoned)?;
    let current = guard.take();
    let new_state = f(current);
    *guard = Some(new_state.clone());
    Ok(new_state)
  }

  fn reset(&self) -> StateResult<()> {
    let mut guard = self.state.write().map_err(|_| StateError::LockPoisoned)?;
    *guard = self.initial.clone();
    Ok(())
  }

  fn is_initialized(&self) -> bool {
    self
      .state
      .read()
      .map(|guard| guard.is_some())
      .unwrap_or(false)
  }

  fn initial_state(&self) -> Option<S> {
    self.initial.clone()
  }
}

/// Configuration for stateful transformers.
#[derive(Debug, Clone)]
pub struct StatefulTransformerConfig<T, S>
where
  T: std::fmt::Debug + Clone + Send + Sync,
  S: Clone + Send + Sync,
{
  /// Base transformer configuration
  pub base: TransformerConfig<T>,
  /// Initial state value
  pub initial_state: Option<S>,
  /// Whether to reset state on pipeline restart
  pub reset_on_restart: bool,
}

impl<T, S> Default for StatefulTransformerConfig<T, S>
where
  T: std::fmt::Debug + Clone + Send + Sync,
  S: Clone + Send + Sync,
{
  fn default() -> Self {
    Self {
      base: TransformerConfig::default(),
      initial_state: None,
      reset_on_restart: true,
    }
  }
}

impl<T, S> StatefulTransformerConfig<T, S>
where
  T: std::fmt::Debug + Clone + Send + Sync,
  S: Clone + Send + Sync,
{
  /// Create a new configuration with an initial state.
  pub fn with_initial_state(mut self, state: S) -> Self {
    self.initial_state = Some(state);
    self
  }

  /// Set whether to reset state on pipeline restart.
  pub fn with_reset_on_restart(mut self, reset: bool) -> Self {
    self.reset_on_restart = reset;
    self
  }

  /// Set the error strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.base = self.base.with_error_strategy(strategy);
    self
  }

  /// Set the transformer name.
  pub fn with_name(mut self, name: String) -> Self {
    self.base = self.base.with_name(name);
    self
  }
}

/// Trait for stateful transformers that maintain state across stream items.
///
/// This trait extends the base [`Transformer`] trait with state management
/// capabilities. Implementations must provide a state store and can use
/// the provided helper methods to access and modify state.
///
/// # State Lifecycle
///
/// 1. **Initialization**: State is initialized when the transformer is created,
///    either with a default value or a provided initial state.
///
/// 2. **Access**: State can be accessed and modified during stream processing
///    using the `state()` and `update_state()` methods.
///
/// 3. **Reset**: State can be reset to its initial value using `reset_state()`.
///
/// 4. **Cleanup**: State is automatically cleaned up when the transformer is dropped.
///
/// # Thread Safety
///
/// All state operations are thread-safe when using [`InMemoryStateStore`].
/// Custom state stores must implement thread-safe access.
///
/// # Example
///
/// ```rust,ignore
/// use crate::stateful_transformer::{StatefulTransformer, InMemoryStateStore, StateStoreExt};
///
/// struct CounterTransformer {
///     state: InMemoryStateStore<usize>,
///     config: TransformerConfig<String>,
/// }
///
/// impl StatefulTransformer for CounterTransformer {
///     type State = usize;
///     type Store = InMemoryStateStore<usize>;
///
///     fn state_store(&self) -> &Self::Store {
///         &self.state
///     }
///
///     fn state_store_mut(&mut self) -> &mut Self::Store {
///         &mut self.state
///     }
/// }
///
/// // Usage:
/// let mut transformer = CounterTransformer::new();
/// transformer.update_state(|count| count.unwrap_or(0) + 1);
/// ```
pub trait StatefulTransformer: Transformer
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The type of state maintained by this transformer.
  type State: Clone + Send + Sync + 'static;

  /// The type of state store used by this transformer.
  type Store: StateStore<Self::State>;

  /// Get a reference to the state store.
  fn state_store(&self) -> &Self::Store;

  /// Get a mutable reference to the state store.
  fn state_store_mut(&mut self) -> &mut Self::Store;

  /// Get the current state value.
  ///
  /// Returns `None` if the state has not been initialized.
  fn state(&self) -> StateResult<Option<Self::State>> {
    self.state_store().get()
  }

  /// Get the current state or the initial state if not set.
  ///
  /// Returns an error if neither current nor initial state is available.
  fn state_or_initial(&self) -> StateResult<Self::State> {
    let store = self.state_store();
    store
      .get()?
      .or_else(|| store.initial_state())
      .ok_or(StateError::NotInitialized)
  }

  /// Update the state using a closure.
  ///
  /// The closure receives the current state (if any) and returns the new state.
  fn update_state<F>(&self, f: F) -> StateResult<Self::State>
  where
    F: FnOnce(Option<Self::State>) -> Self::State + Send + 'static,
  {
    self.state_store().update_with(Box::new(f))
  }

  /// Set the state to a new value.
  fn set_state(&self, state: Self::State) -> StateResult<()> {
    self.state_store().set(state)
  }

  /// Reset the state to its initial value.
  fn reset_state(&self) -> StateResult<()> {
    self.state_store().reset()
  }

  /// Check if the state has been initialized.
  fn has_state(&self) -> bool {
    self.state_store().is_initialized()
  }
}

// ============================================================================
// State Checkpointing
// ============================================================================

/// Error type for checkpoint operations.
#[derive(Debug)]
pub enum StatefulCheckpointError {
  /// State is not initialized and cannot be checkpointed
  NoState,
  /// Serialization failed
  SerializationFailed(String),
  /// Deserialization failed
  DeserializationFailed(String),
  /// I/O error during checkpoint operation
  IoError(std::io::Error),
  /// State store error
  StateError(StateError),
}

impl std::fmt::Display for StatefulCheckpointError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StatefulCheckpointError::NoState => write!(f, "No state to checkpoint"),
      StatefulCheckpointError::SerializationFailed(msg) => {
        write!(f, "Checkpoint serialization failed: {}", msg)
      }
      StatefulCheckpointError::DeserializationFailed(msg) => {
        write!(f, "Checkpoint deserialization failed: {}", msg)
      }
      StatefulCheckpointError::IoError(err) => write!(f, "Checkpoint I/O error: {}", err),
      StatefulCheckpointError::StateError(err) => {
        write!(f, "State error during checkpoint: {}", err)
      }
    }
  }
}

impl std::error::Error for StatefulCheckpointError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      StatefulCheckpointError::IoError(err) => Some(err),
      _ => None,
    }
  }
}

impl From<std::io::Error> for StatefulCheckpointError {
  fn from(err: std::io::Error) -> Self {
    StatefulCheckpointError::IoError(err)
  }
}

impl From<StateError> for StatefulCheckpointError {
  fn from(err: StateError) -> Self {
    StatefulCheckpointError::StateError(err)
  }
}

/// Result type for checkpoint operations.
pub type StatefulCheckpointResult<T> = Result<T, StatefulCheckpointError>;

/// Configuration for state checkpointing.
#[derive(Debug, Clone)]
pub struct StatefulCheckpointConfig {
  /// Interval between automatic checkpoints (number of items processed).
  /// Set to 0 to disable automatic checkpointing.
  pub checkpoint_interval: usize,
  /// Whether to checkpoint on pipeline completion.
  pub checkpoint_on_complete: bool,
  /// Whether to restore from checkpoint on startup.
  pub restore_on_startup: bool,
}

impl Default for StatefulCheckpointConfig {
  fn default() -> Self {
    Self {
      checkpoint_interval: 0, // Disabled by default
      checkpoint_on_complete: true,
      restore_on_startup: true,
    }
  }
}

impl StatefulCheckpointConfig {
  /// Create a new checkpoint configuration with the specified interval.
  pub fn with_interval(interval: usize) -> Self {
    Self {
      checkpoint_interval: interval,
      ..Default::default()
    }
  }

  /// Set whether to checkpoint on pipeline completion.
  pub fn checkpoint_on_complete(mut self, enable: bool) -> Self {
    self.checkpoint_on_complete = enable;
    self
  }

  /// Set whether to restore from checkpoint on startup.
  pub fn restore_on_startup(mut self, enable: bool) -> Self {
    self.restore_on_startup = enable;
    self
  }

  /// Check if automatic checkpointing is enabled.
  pub fn is_auto_checkpoint_enabled(&self) -> bool {
    self.checkpoint_interval > 0
  }
}

/// Trait for checkpoint storage backends.
///
/// This trait abstracts the persistence mechanism for state checkpoints,
/// allowing for different implementations (file, database, cloud storage, etc.).
pub trait StatefulCheckpointStore: Send + Sync {
  /// Save a checkpoint with the given data.
  fn save(&self, data: &[u8]) -> StatefulCheckpointResult<()>;

  /// Load the most recent checkpoint.
  ///
  /// Returns `None` if no checkpoint exists.
  fn load(&self) -> StatefulCheckpointResult<Option<Vec<u8>>>;

  /// Delete all checkpoints.
  fn clear(&self) -> StatefulCheckpointResult<()>;

  /// Check if a checkpoint exists.
  fn exists(&self) -> bool;
}

/// File-based checkpoint store.
///
/// Saves state checkpoints to a file on the local filesystem.
///
/// # Example
///
/// ```rust,no_run
/// use crate::stateful_transformer::{FileStatefulCheckpointStore, StatefulCheckpointStore};
/// use std::path::PathBuf;
///
/// let store = FileStatefulCheckpointStore::new(PathBuf::from("/tmp/my_checkpoint.json"));
/// store.save(b"{\"count\": 42}").unwrap();
///
/// let data = store.load().unwrap();
/// assert!(data.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct FileStatefulCheckpointStore {
  path: std::path::PathBuf,
}

impl FileStatefulCheckpointStore {
  /// Create a new file checkpoint store with the specified path.
  pub fn new(path: std::path::PathBuf) -> Self {
    Self { path }
  }

  /// Get the checkpoint file path.
  pub fn path(&self) -> &std::path::Path {
    &self.path
  }
}

impl StatefulCheckpointStore for FileStatefulCheckpointStore {
  fn save(&self, data: &[u8]) -> StatefulCheckpointResult<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = self.path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    // Write to a temporary file first, then rename for atomicity
    let temp_path = self.path.with_extension("tmp");
    std::fs::write(&temp_path, data)?;
    std::fs::rename(&temp_path, &self.path)?;

    Ok(())
  }

  fn load(&self) -> StatefulCheckpointResult<Option<Vec<u8>>> {
    if !self.path.exists() {
      return Ok(None);
    }

    let data = std::fs::read(&self.path)?;
    Ok(Some(data))
  }

  fn clear(&self) -> StatefulCheckpointResult<()> {
    if self.path.exists() {
      std::fs::remove_file(&self.path)?;
    }
    Ok(())
  }

  fn exists(&self) -> bool {
    self.path.exists()
  }
}

/// In-memory checkpoint store for testing.
///
/// Stores checkpoints in memory without persistence.
#[derive(Debug, Default)]
pub struct InMemoryStatefulCheckpointStore {
  data: std::sync::RwLock<Option<Vec<u8>>>,
}

impl InMemoryStatefulCheckpointStore {
  /// Create a new in-memory checkpoint store.
  pub fn new() -> Self {
    Self::default()
  }
}

impl StatefulCheckpointStore for InMemoryStatefulCheckpointStore {
  fn save(&self, data: &[u8]) -> StatefulCheckpointResult<()> {
    let mut guard = self
      .data
      .write()
      .map_err(|_| StatefulCheckpointError::SerializationFailed("Lock poisoned".to_string()))?;
    *guard = Some(data.to_vec());
    Ok(())
  }

  fn load(&self) -> StatefulCheckpointResult<Option<Vec<u8>>> {
    let guard = self
      .data
      .read()
      .map_err(|_| StatefulCheckpointError::DeserializationFailed("Lock poisoned".to_string()))?;
    Ok(guard.clone())
  }

  fn clear(&self) -> StatefulCheckpointResult<()> {
    let mut guard = self
      .data
      .write()
      .map_err(|_| StatefulCheckpointError::SerializationFailed("Lock poisoned".to_string()))?;
    *guard = None;
    Ok(())
  }

  fn exists(&self) -> bool {
    self.data.read().map(|g| g.is_some()).unwrap_or(false)
  }
}

/// Extension trait for checkpointing state stores with serde-serializable state.
///
/// This trait provides checkpoint functionality for any state store where the
/// state type implements `serde::Serialize` and `serde::Deserialize`.
pub trait StatefulCheckpointableStateStore<S>: StateStore<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  /// Create a JSON checkpoint of the current state.
  fn create_json_checkpoint(&self) -> StatefulCheckpointResult<Vec<u8>> {
    let state = self.get()?.ok_or(StatefulCheckpointError::NoState)?;
    serde_json::to_vec(&state)
      .map_err(|e| StatefulCheckpointError::SerializationFailed(e.to_string()))
  }

  /// Restore state from a JSON checkpoint.
  fn restore_from_json_checkpoint(&self, data: &[u8]) -> StatefulCheckpointResult<()> {
    let state: S = serde_json::from_slice(data)
      .map_err(|e| StatefulCheckpointError::DeserializationFailed(e.to_string()))?;
    self.set(state)?;
    Ok(())
  }

  /// Create a pretty-printed JSON checkpoint of the current state.
  fn create_json_checkpoint_pretty(&self) -> StatefulCheckpointResult<Vec<u8>> {
    let state = self.get()?.ok_or(StatefulCheckpointError::NoState)?;
    serde_json::to_vec_pretty(&state)
      .map_err(|e| StatefulCheckpointError::SerializationFailed(e.to_string()))
  }

  /// Save the current state to a checkpoint store.
  fn save_checkpoint(
    &self,
    checkpoint_store: &dyn StatefulCheckpointStore,
  ) -> StatefulCheckpointResult<()> {
    let data = self.create_json_checkpoint()?;
    checkpoint_store.save(&data)
  }

  /// Load state from a checkpoint store.
  fn load_checkpoint(
    &self,
    checkpoint_store: &dyn StatefulCheckpointStore,
  ) -> StatefulCheckpointResult<bool> {
    match checkpoint_store.load()? {
      Some(data) => {
        self.restore_from_json_checkpoint(&data)?;
        Ok(true)
      }
      None => Ok(false),
    }
  }
}

// Blanket implementation for all StateStore types with serializable state
impl<S, T> StatefulCheckpointableStateStore<S> for T
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
  T: StateStore<S>,
{
}

/// Helper struct for managing checkpoints with automatic intervals.
pub struct StatefulCheckpointManager<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  store: Box<dyn StatefulCheckpointStore>,
  config: StatefulCheckpointConfig,
  items_since_checkpoint: std::sync::atomic::AtomicUsize,
  _phantom: std::marker::PhantomData<S>,
}

impl<S> std::fmt::Debug for StatefulCheckpointManager<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("StatefulCheckpointManager")
      .field("store", &"<dyn StatefulCheckpointStore>")
      .field("config", &self.config)
      .field("items_since_checkpoint", &self.items_since_checkpoint)
      .finish()
  }
}

impl<S> StatefulCheckpointManager<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  /// Create a new checkpoint manager with the given store and configuration.
  pub fn new(store: Box<dyn StatefulCheckpointStore>, config: StatefulCheckpointConfig) -> Self {
    Self {
      store,
      config,
      items_since_checkpoint: std::sync::atomic::AtomicUsize::new(0),
      _phantom: std::marker::PhantomData,
    }
  }

  /// Create a checkpoint manager with a file store.
  pub fn with_file(path: std::path::PathBuf, config: StatefulCheckpointConfig) -> Self {
    Self::new(Box::new(FileStatefulCheckpointStore::new(path)), config)
  }

  /// Get the checkpoint configuration.
  pub fn config(&self) -> &StatefulCheckpointConfig {
    &self.config
  }

  /// Record that an item has been processed and potentially trigger a checkpoint.
  ///
  /// Returns `true` if a checkpoint should be taken based on the interval.
  pub fn record_item(&self) -> bool {
    if !self.config.is_auto_checkpoint_enabled() {
      return false;
    }

    let count = self
      .items_since_checkpoint
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    count + 1 >= self.config.checkpoint_interval
  }

  /// Reset the item counter after a checkpoint.
  pub fn reset_counter(&self) {
    self
      .items_since_checkpoint
      .store(0, std::sync::atomic::Ordering::Relaxed);
  }

  /// Save state to the checkpoint store.
  pub fn save<Store>(&self, state_store: &Store) -> StatefulCheckpointResult<()>
  where
    Store: StatefulCheckpointableStateStore<S>,
  {
    state_store.save_checkpoint(self.store.as_ref())?;
    self.reset_counter();
    Ok(())
  }

  /// Load state from the checkpoint store.
  pub fn load<Store>(&self, state_store: &Store) -> StatefulCheckpointResult<bool>
  where
    Store: StatefulCheckpointableStateStore<S>,
  {
    state_store.load_checkpoint(self.store.as_ref())
  }

  /// Clear all checkpoints.
  pub fn clear(&self) -> StatefulCheckpointResult<()> {
    self.store.clear()
  }

  /// Check if a checkpoint exists.
  pub fn has_checkpoint(&self) -> bool {
    self.store.exists()
  }

  /// Conditionally save a checkpoint if the interval has been reached.
  pub fn maybe_checkpoint<Store>(&self, state_store: &Store) -> StatefulCheckpointResult<()>
  where
    Store: StatefulCheckpointableStateStore<S>,
  {
    if self.record_item() {
      self.save(state_store)?;
    }
    Ok(())
  }
}
