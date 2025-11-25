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
}
