//! Stateful transformer trait for stream processing with persistent state.
//!
//! This module provides the [`StatefulTransformer`] trait which extends the base
//! [`Transformer`](crate::transformer::Transformer) trait with state management capabilities.
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
//! use streamweave::stateful_transformer::{StatefulTransformer, InMemoryStateStore};
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

use crate::transformer::Transformer;

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
/// use streamweave::stateful_transformer::{InMemoryStateStore, StateStore, StateCheckpoint};
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
/// use streamweave::stateful_transformer::{InMemoryStateStore, StateStore, StateStoreExt};
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
  pub base: crate::transformer::TransformerConfig<T>,
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
      base: crate::transformer::TransformerConfig::default(),
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
  pub fn with_error_strategy(mut self, strategy: crate::error::ErrorStrategy<T>) -> Self {
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
/// use streamweave::stateful_transformer::{StatefulTransformer, InMemoryStateStore, StateStoreExt};
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
pub enum CheckpointError {
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

impl std::fmt::Display for CheckpointError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CheckpointError::NoState => write!(f, "No state to checkpoint"),
      CheckpointError::SerializationFailed(msg) => {
        write!(f, "Checkpoint serialization failed: {}", msg)
      }
      CheckpointError::DeserializationFailed(msg) => {
        write!(f, "Checkpoint deserialization failed: {}", msg)
      }
      CheckpointError::IoError(err) => write!(f, "Checkpoint I/O error: {}", err),
      CheckpointError::StateError(err) => write!(f, "State error during checkpoint: {}", err),
    }
  }
}

impl std::error::Error for CheckpointError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      CheckpointError::IoError(err) => Some(err),
      _ => None,
    }
  }
}

impl From<std::io::Error> for CheckpointError {
  fn from(err: std::io::Error) -> Self {
    CheckpointError::IoError(err)
  }
}

impl From<StateError> for CheckpointError {
  fn from(err: StateError) -> Self {
    CheckpointError::StateError(err)
  }
}

/// Result type for checkpoint operations.
pub type CheckpointResult<T> = Result<T, CheckpointError>;

/// Configuration for state checkpointing.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
  /// Interval between automatic checkpoints (number of items processed).
  /// Set to 0 to disable automatic checkpointing.
  pub checkpoint_interval: usize,
  /// Whether to checkpoint on pipeline completion.
  pub checkpoint_on_complete: bool,
  /// Whether to restore from checkpoint on startup.
  pub restore_on_startup: bool,
}

impl Default for CheckpointConfig {
  fn default() -> Self {
    Self {
      checkpoint_interval: 0, // Disabled by default
      checkpoint_on_complete: true,
      restore_on_startup: true,
    }
  }
}

impl CheckpointConfig {
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
pub trait CheckpointStore: Send + Sync {
  /// Save a checkpoint with the given data.
  fn save(&self, data: &[u8]) -> CheckpointResult<()>;

  /// Load the most recent checkpoint.
  ///
  /// Returns `None` if no checkpoint exists.
  fn load(&self) -> CheckpointResult<Option<Vec<u8>>>;

  /// Delete all checkpoints.
  fn clear(&self) -> CheckpointResult<()>;

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
/// use streamweave::stateful_transformer::{FileCheckpointStore, CheckpointStore};
/// use std::path::PathBuf;
///
/// let store = FileCheckpointStore::new(PathBuf::from("/tmp/my_checkpoint.json"));
/// store.save(b"{\"count\": 42}").unwrap();
///
/// let data = store.load().unwrap();
/// assert!(data.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct FileCheckpointStore {
  path: std::path::PathBuf,
}

impl FileCheckpointStore {
  /// Create a new file checkpoint store with the specified path.
  pub fn new(path: std::path::PathBuf) -> Self {
    Self { path }
  }

  /// Get the checkpoint file path.
  pub fn path(&self) -> &std::path::Path {
    &self.path
  }
}

impl CheckpointStore for FileCheckpointStore {
  fn save(&self, data: &[u8]) -> CheckpointResult<()> {
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

  fn load(&self) -> CheckpointResult<Option<Vec<u8>>> {
    if !self.path.exists() {
      return Ok(None);
    }

    let data = std::fs::read(&self.path)?;
    Ok(Some(data))
  }

  fn clear(&self) -> CheckpointResult<()> {
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
pub struct InMemoryCheckpointStore {
  data: std::sync::RwLock<Option<Vec<u8>>>,
}

impl InMemoryCheckpointStore {
  /// Create a new in-memory checkpoint store.
  pub fn new() -> Self {
    Self::default()
  }
}

impl CheckpointStore for InMemoryCheckpointStore {
  fn save(&self, data: &[u8]) -> CheckpointResult<()> {
    let mut guard = self
      .data
      .write()
      .map_err(|_| CheckpointError::SerializationFailed("Lock poisoned".to_string()))?;
    *guard = Some(data.to_vec());
    Ok(())
  }

  fn load(&self) -> CheckpointResult<Option<Vec<u8>>> {
    let guard = self
      .data
      .read()
      .map_err(|_| CheckpointError::DeserializationFailed("Lock poisoned".to_string()))?;
    Ok(guard.clone())
  }

  fn clear(&self) -> CheckpointResult<()> {
    let mut guard = self
      .data
      .write()
      .map_err(|_| CheckpointError::SerializationFailed("Lock poisoned".to_string()))?;
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
pub trait CheckpointableStateStore<S>: StateStore<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  /// Create a JSON checkpoint of the current state.
  fn create_json_checkpoint(&self) -> CheckpointResult<Vec<u8>> {
    let state = self.get()?.ok_or(CheckpointError::NoState)?;
    serde_json::to_vec(&state).map_err(|e| CheckpointError::SerializationFailed(e.to_string()))
  }

  /// Restore state from a JSON checkpoint.
  fn restore_from_json_checkpoint(&self, data: &[u8]) -> CheckpointResult<()> {
    let state: S = serde_json::from_slice(data)
      .map_err(|e| CheckpointError::DeserializationFailed(e.to_string()))?;
    self.set(state)?;
    Ok(())
  }

  /// Create a pretty-printed JSON checkpoint of the current state.
  fn create_json_checkpoint_pretty(&self) -> CheckpointResult<Vec<u8>> {
    let state = self.get()?.ok_or(CheckpointError::NoState)?;
    serde_json::to_vec_pretty(&state)
      .map_err(|e| CheckpointError::SerializationFailed(e.to_string()))
  }

  /// Save the current state to a checkpoint store.
  fn save_checkpoint(&self, checkpoint_store: &dyn CheckpointStore) -> CheckpointResult<()> {
    let data = self.create_json_checkpoint()?;
    checkpoint_store.save(&data)
  }

  /// Load state from a checkpoint store.
  fn load_checkpoint(&self, checkpoint_store: &dyn CheckpointStore) -> CheckpointResult<bool> {
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
impl<S, T> CheckpointableStateStore<S> for T
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
  T: StateStore<S>,
{
}

/// Helper struct for managing checkpoints with automatic intervals.
pub struct CheckpointManager<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  store: Box<dyn CheckpointStore>,
  config: CheckpointConfig,
  items_since_checkpoint: std::sync::atomic::AtomicUsize,
  _phantom: std::marker::PhantomData<S>,
}

impl<S> std::fmt::Debug for CheckpointManager<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CheckpointManager")
      .field("store", &"<dyn CheckpointStore>")
      .field("config", &self.config)
      .field("items_since_checkpoint", &self.items_since_checkpoint)
      .finish()
  }
}

impl<S> CheckpointManager<S>
where
  S: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
  /// Create a new checkpoint manager with the given store and configuration.
  pub fn new(store: Box<dyn CheckpointStore>, config: CheckpointConfig) -> Self {
    Self {
      store,
      config,
      items_since_checkpoint: std::sync::atomic::AtomicUsize::new(0),
      _phantom: std::marker::PhantomData,
    }
  }

  /// Create a checkpoint manager with a file store.
  pub fn with_file(path: std::path::PathBuf, config: CheckpointConfig) -> Self {
    Self::new(Box::new(FileCheckpointStore::new(path)), config)
  }

  /// Get the checkpoint configuration.
  pub fn config(&self) -> &CheckpointConfig {
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
  pub fn save<Store>(&self, state_store: &Store) -> CheckpointResult<()>
  where
    Store: CheckpointableStateStore<S>,
  {
    state_store.save_checkpoint(self.store.as_ref())?;
    self.reset_counter();
    Ok(())
  }

  /// Load state from the checkpoint store.
  pub fn load<Store>(&self, state_store: &Store) -> CheckpointResult<bool>
  where
    Store: CheckpointableStateStore<S>,
  {
    state_store.load_checkpoint(self.store.as_ref())
  }

  /// Clear all checkpoints.
  pub fn clear(&self) -> CheckpointResult<()> {
    self.store.clear()
  }

  /// Check if a checkpoint exists.
  pub fn has_checkpoint(&self) -> bool {
    self.store.exists()
  }

  /// Conditionally save a checkpoint if the interval has been reached.
  pub fn maybe_checkpoint<Store>(&self, state_store: &Store) -> CheckpointResult<()>
  where
    Store: CheckpointableStateStore<S>,
  {
    if self.record_item() {
      self.save(state_store)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_in_memory_state_store_new() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
    assert!(store.is_initialized());
    assert_eq!(store.get().unwrap(), Some(42));
    assert_eq!(store.initial_state(), Some(42));
  }

  #[test]
  fn test_in_memory_state_store_empty() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    assert!(!store.is_initialized());
    assert_eq!(store.get().unwrap(), None);
    assert_eq!(store.initial_state(), None);
  }

  #[test]
  fn test_in_memory_state_store_set() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    assert!(!store.is_initialized());

    store.set(100).unwrap();
    assert!(store.is_initialized());
    assert_eq!(store.get().unwrap(), Some(100));
  }

  #[test]
  fn test_in_memory_state_store_update() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(10);

    let result = store.update(|current| current.unwrap_or(0) + 5).unwrap();
    assert_eq!(result, 15);
    assert_eq!(store.get().unwrap(), Some(15));
  }

  #[test]
  fn test_in_memory_state_store_update_from_empty() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();

    let result = store.update(|current| current.unwrap_or(100)).unwrap();
    assert_eq!(result, 100);
    assert_eq!(store.get().unwrap(), Some(100));
  }

  #[test]
  fn test_in_memory_state_store_reset() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);

    store.set(100).unwrap();
    assert_eq!(store.get().unwrap(), Some(100));

    store.reset().unwrap();
    assert_eq!(store.get().unwrap(), Some(42)); // Back to initial
  }

  #[test]
  fn test_in_memory_state_store_reset_empty() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();

    store.set(100).unwrap();
    assert_eq!(store.get().unwrap(), Some(100));

    store.reset().unwrap();
    assert_eq!(store.get().unwrap(), None); // No initial, so None
  }

  #[test]
  fn test_in_memory_state_store_clone() {
    let store1: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
    store1.set(100).unwrap();

    let store2 = store1.clone();

    // Cloned store has same state value
    assert_eq!(store2.get().unwrap(), Some(100));

    // But modifying one doesn't affect the other
    store1.set(200).unwrap();
    assert_eq!(store1.get().unwrap(), Some(200));
    assert_eq!(store2.get().unwrap(), Some(100));
  }

  #[test]
  fn test_in_memory_state_store_default() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::default();
    assert!(store.is_initialized());
    assert_eq!(store.get().unwrap(), Some(0));
  }

  #[test]
  fn test_in_memory_state_store_with_string() {
    let store: InMemoryStateStore<String> = InMemoryStateStore::new("hello".to_string());
    assert_eq!(store.get().unwrap(), Some("hello".to_string()));

    store.set("world".to_string()).unwrap();
    assert_eq!(store.get().unwrap(), Some("world".to_string()));
  }

  #[test]
  fn test_in_memory_state_store_with_vec() {
    let store: InMemoryStateStore<Vec<i32>> = InMemoryStateStore::new(vec![1, 2, 3]);
    assert_eq!(store.get().unwrap(), Some(vec![1, 2, 3]));

    store
      .update(|current| {
        let mut v = current.unwrap_or_default();
        v.push(4);
        v
      })
      .unwrap();
    assert_eq!(store.get().unwrap(), Some(vec![1, 2, 3, 4]));
  }

  #[test]
  fn test_stateful_transformer_config_default() {
    let config: StatefulTransformerConfig<i32, i64> = StatefulTransformerConfig::default();
    assert!(config.initial_state.is_none());
    assert!(config.reset_on_restart);
  }

  #[test]
  fn test_stateful_transformer_config_with_initial_state() {
    let config: StatefulTransformerConfig<i32, i64> =
      StatefulTransformerConfig::default().with_initial_state(100);
    assert_eq!(config.initial_state, Some(100));
  }

  #[test]
  fn test_stateful_transformer_config_with_reset_on_restart() {
    let config: StatefulTransformerConfig<i32, i64> =
      StatefulTransformerConfig::default().with_reset_on_restart(false);
    assert!(!config.reset_on_restart);
  }

  #[test]
  fn test_stateful_transformer_config_with_name() {
    let config: StatefulTransformerConfig<i32, i64> =
      StatefulTransformerConfig::default().with_name("test".to_string());
    assert_eq!(config.base.name, Some("test".to_string()));
  }

  #[test]
  fn test_state_error_display() {
    assert_eq!(
      format!("{}", StateError::NotInitialized),
      "State is not initialized"
    );
    assert_eq!(
      format!("{}", StateError::LockPoisoned),
      "State lock is poisoned"
    );
    assert_eq!(
      format!("{}", StateError::UpdateFailed("oops".to_string())),
      "State update failed: oops"
    );
    assert_eq!(
      format!("{}", StateError::SerializationFailed("bad".to_string())),
      "State serialization failed: bad"
    );
    assert_eq!(
      format!("{}", StateError::DeserializationFailed("bad".to_string())),
      "State deserialization failed: bad"
    );
  }

  #[test]
  fn test_concurrent_state_access() {
    use std::sync::Arc;
    use std::thread;

    let store = Arc::new(InMemoryStateStore::new(0i64));
    let mut handles = vec![];

    // Spawn multiple threads that increment the state
    for _ in 0..10 {
      let store_clone = Arc::clone(&store);
      handles.push(thread::spawn(move || {
        for _ in 0..100 {
          store_clone
            .update(|current| current.unwrap_or(0) + 1)
            .unwrap();
        }
      }));
    }

    // Wait for all threads to complete
    for handle in handles {
      handle.join().unwrap();
    }

    // All increments should have been applied
    assert_eq!(store.get().unwrap(), Some(1000));
  }

  #[test]
  fn test_state_store_read_guard() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
    {
      let guard = store.read().unwrap();
      assert_eq!(*guard, Some(42));
    }
    // Guard is dropped, we can write now
    store.set(100).unwrap();
    assert_eq!(store.get().unwrap(), Some(100));
  }

  #[test]
  fn test_state_store_write_guard() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
    {
      let mut guard = store.write().unwrap();
      *guard = Some(100);
    }
    assert_eq!(store.get().unwrap(), Some(100));
  }

  #[test]
  fn test_with_optional_initial_some() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::with_optional_initial(Some(42));
    assert!(store.is_initialized());
    assert_eq!(store.get().unwrap(), Some(42));
    assert_eq!(store.initial_state(), Some(42));
  }

  #[test]
  fn test_with_optional_initial_none() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::with_optional_initial(None);
    assert!(!store.is_initialized());
    assert_eq!(store.get().unwrap(), None);
    assert_eq!(store.initial_state(), None);
  }

  // Checkpointing tests

  #[test]
  fn test_serialize_state_with_value() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
    let serialized = store.serialize_state().unwrap();
    assert!(!serialized.is_empty());

    // Verify it's valid JSON
    let value: i64 = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(value, 42);
  }

  #[test]
  fn test_serialize_state_empty() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    let serialized = store.serialize_state().unwrap();
    assert!(serialized.is_empty());
  }

  #[test]
  fn test_deserialize_and_set_state() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    let data = serde_json::to_vec(&100i64).unwrap();

    store.deserialize_and_set_state(&data).unwrap();
    assert_eq!(store.get().unwrap(), Some(100));
  }

  #[test]
  fn test_checkpoint_roundtrip() {
    // Create store with initial value and modify it
    let store1: InMemoryStateStore<i64> = InMemoryStateStore::new(10);
    store1.set(42).unwrap();

    // Serialize the state
    let checkpoint = store1.serialize_state().unwrap();

    // Create a new store and restore from checkpoint
    let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    store2.deserialize_and_set_state(&checkpoint).unwrap();

    // Verify state was restored
    assert_eq!(store2.get().unwrap(), Some(42));
  }

  #[test]
  fn test_checkpoint_complex_type() {
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
    struct ComplexState {
      count: i32,
      values: Vec<String>,
    }

    let store: InMemoryStateStore<ComplexState> = InMemoryStateStore::new(ComplexState {
      count: 5,
      values: vec!["a".to_string(), "b".to_string()],
    });

    // Serialize
    let checkpoint = store.serialize_state().unwrap();

    // Restore to new store
    let store2: InMemoryStateStore<ComplexState> = InMemoryStateStore::empty();
    store2.deserialize_and_set_state(&checkpoint).unwrap();

    let restored = store2.get().unwrap().unwrap();
    assert_eq!(restored.count, 5);
    assert_eq!(restored.values, vec!["a".to_string(), "b".to_string()]);
  }

  #[test]
  fn test_deserialize_invalid_data() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    let invalid_data = b"not valid json";

    let result = store.deserialize_and_set_state(invalid_data);
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      StateError::DeserializationFailed(_)
    ));
  }

  #[test]
  fn test_checkpoint_after_updates() {
    let store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);

    // Perform several updates
    for i in 1..=10 {
      store
        .update(move |current| current.unwrap_or(0) + i)
        .unwrap();
    }

    // State should be 1+2+...+10 = 55
    assert_eq!(store.get().unwrap(), Some(55));

    // Serialize and restore
    let checkpoint = store.serialize_state().unwrap();
    let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
    store2.deserialize_and_set_state(&checkpoint).unwrap();

    // Verify the accumulated state was preserved
    assert_eq!(store2.get().unwrap(), Some(55));
  }
}
