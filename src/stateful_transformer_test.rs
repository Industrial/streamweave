//! Tests for stateful transformer module.

use crate::error::ErrorStrategy;
use crate::input::Input;
use crate::message::{Message, MessageId};
use crate::output::Output;
use crate::stateful_transformer::*;
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc as StdArc;
use tokio_stream::StreamExt;

// ============================================================================
// Mock Stateful Transformer for Testing
// ============================================================================

/// Mock stateful transformer that implements StatefulTransformer for testing.
struct MockStatefulTransformer {
  state: StdArc<InMemoryStateStore<i64>>,
  config: TransformerConfig<Message<i32>>,
}

impl MockStatefulTransformer {
  fn new(initial_state: i64) -> Self {
    Self {
      state: StdArc::new(InMemoryStateStore::new(initial_state)),
      config: TransformerConfig::default(),
    }
  }
}

impl Clone for MockStatefulTransformer {
  fn clone(&self) -> Self {
    Self {
      state: StdArc::clone(&self.state),
      config: self.config.clone(),
    }
  }
}

impl Input for MockStatefulTransformer {
  type Input = Message<i32>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

impl Output for MockStatefulTransformer {
  type Output = Message<i64>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<i64>> + Send>>;
}

#[async_trait]
impl Transformer for MockStatefulTransformer {
  type InputPorts = (Message<i32>,);
  type OutputPorts = (Message<i64>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // Clone the Arc to share the store across closures
    let state_store = StdArc::clone(&self.state);
    Box::pin(input.map(move |msg| {
      // Extract payload value before moving into closure
      let payload_value = *msg.payload();
      let id = msg.id().clone();
      let metadata = msg.metadata().clone();
      // Update state atomically
      let payload = payload_value; // Move into a variable that can be captured
      let new_value = state_store
        .update(move |current| current.unwrap_or(0) + payload as i64)
        .unwrap();
      Message::with_metadata(new_value, id, metadata)
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }
}

impl StatefulTransformer for MockStatefulTransformer {
  type State = i64;
  type Store = InMemoryStateStore<i64>;

  fn state_store(&self) -> &Self::Store {
    &self.state
  }

  fn state_store_mut(&mut self) -> &mut Self::Store {
    StdArc::get_mut(&mut self.state).unwrap()
  }
}

// ============================================================================
// StateError Tests
// ============================================================================

#[test]
fn test_state_error_display() {
  let err = StateError::NotInitialized;
  assert_eq!(format!("{}", err), "State is not initialized");

  let err = StateError::LockPoisoned;
  assert_eq!(format!("{}", err), "State lock is poisoned");

  let err = StateError::UpdateFailed("test error".to_string());
  assert_eq!(format!("{}", err), "State update failed: test error");

  let err = StateError::SerializationFailed("json error".to_string());
  assert_eq!(format!("{}", err), "State serialization failed: json error");

  let err = StateError::DeserializationFailed("parse error".to_string());
  assert_eq!(
    format!("{}", err),
    "State deserialization failed: parse error"
  );
}

#[test]
fn test_state_error_debug() {
  let err = StateError::NotInitialized;
  let debug_str = format!("{:?}", err);
  assert!(debug_str.contains("NotInitialized"));
}

#[test]
fn test_state_error_clone() {
  let err = StateError::UpdateFailed("error".to_string());
  let cloned = err.clone();
  assert_eq!(format!("{}", err), format!("{}", cloned));
}

#[test]
fn test_state_error_is_error() {
  let err = StateError::NotInitialized;
  assert!(err.source().is_none());
}

// ============================================================================
// InMemoryStateStore Tests
// ============================================================================

#[test]
fn test_in_memory_state_store_new() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  assert_eq!(store.get().unwrap(), Some(42));
  assert_eq!(store.initial_state(), Some(42));
  assert!(store.is_initialized());
}

#[test]
fn test_in_memory_state_store_empty() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  assert_eq!(store.get().unwrap(), None);
  assert_eq!(store.initial_state(), None);
  assert!(!store.is_initialized());
}

#[test]
fn test_in_memory_state_store_with_optional_initial() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::with_optional_initial(Some(100));
  assert_eq!(store.get().unwrap(), Some(100));
  assert_eq!(store.initial_state(), Some(100));

  let store: InMemoryStateStore<i64> = InMemoryStateStore::with_optional_initial(None);
  assert_eq!(store.get().unwrap(), None);
  assert_eq!(store.initial_state(), None);
}

#[test]
fn test_in_memory_state_store_set() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  assert_eq!(store.get().unwrap(), None);

  store.set(42).unwrap();
  assert_eq!(store.get().unwrap(), Some(42));
}

#[test]
fn test_in_memory_state_store_update_with() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);
  let new_state = store
    .update_with(Box::new(|current| current.unwrap_or(0) + 10))
    .unwrap();
  assert_eq!(new_state, 10);
  assert_eq!(store.get().unwrap(), Some(10));
}

#[test]
fn test_in_memory_state_store_update_with_none() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let new_state = store
    .update_with(Box::new(|current| current.unwrap_or(100)))
    .unwrap();
  assert_eq!(new_state, 100);
  assert_eq!(store.get().unwrap(), Some(100));
}

#[test]
fn test_in_memory_state_store_reset() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  store.set(100).unwrap();
  assert_eq!(store.get().unwrap(), Some(100));

  store.reset().unwrap();
  assert_eq!(store.get().unwrap(), Some(42));
}

#[test]
fn test_in_memory_state_store_reset_empty() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  store.set(100).unwrap();
  assert_eq!(store.get().unwrap(), Some(100));

  store.reset().unwrap();
  assert_eq!(store.get().unwrap(), None);
}

#[test]
fn test_in_memory_state_store_clone() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  store.set(100).unwrap();

  let cloned = store.clone();
  assert_eq!(cloned.get().unwrap(), Some(100));
  assert_eq!(cloned.initial_state(), Some(42));

  // Mutations should be independent
  store.set(200).unwrap();
  assert_eq!(store.get().unwrap(), Some(200));
  assert_eq!(cloned.get().unwrap(), Some(100));
}

#[test]
fn test_in_memory_state_store_default() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::default();
  assert_eq!(store.get().unwrap(), Some(0));
  assert_eq!(store.initial_state(), Some(0));
}

#[test]
fn test_in_memory_state_store_read_write() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let read_guard = store.read().unwrap();
  assert_eq!(*read_guard, Some(42));
  drop(read_guard);

  let mut write_guard = store.write().unwrap();
  *write_guard = Some(100);
  drop(write_guard);

  assert_eq!(store.get().unwrap(), Some(100));
}

// ============================================================================
// StateStoreExt Tests
// ============================================================================

#[test]
fn test_state_store_ext_update() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);
  let new_state = store.update(|current| current.unwrap_or(0) + 5).unwrap();
  assert_eq!(new_state, 5);
  assert_eq!(store.get().unwrap(), Some(5));
}

// ============================================================================
// StateCheckpoint Tests
// ============================================================================

#[test]
fn test_state_checkpoint_serialize_state() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let data = store.serialize_state().unwrap();
  assert!(!data.is_empty());

  let restored: i64 = serde_json::from_slice(&data).unwrap();
  assert_eq!(restored, 42);
}

#[test]
fn test_state_checkpoint_serialize_state_empty() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let data = store.serialize_state().unwrap();
  assert!(data.is_empty());
}

#[test]
fn test_state_checkpoint_deserialize_and_set_state() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let data = serde_json::to_vec(&42i64).unwrap();

  store.deserialize_and_set_state(&data).unwrap();
  assert_eq!(store.get().unwrap(), Some(42));
}

#[test]
fn test_state_checkpoint_deserialize_and_set_state_empty() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  store.deserialize_and_set_state(&[]).unwrap();
  assert_eq!(store.get().unwrap(), Some(0)); // default for i64
}

#[test]
fn test_state_checkpoint_roundtrip() {
  let store1: InMemoryStateStore<i64> = InMemoryStateStore::new(123);
  let data = store1.serialize_state().unwrap();

  let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  store2.deserialize_and_set_state(&data).unwrap();

  assert_eq!(store1.get().unwrap(), store2.get().unwrap());
}

// ============================================================================
// StatefulTransformerConfig Tests
// ============================================================================

#[test]
fn test_stateful_transformer_config_default() {
  let config: StatefulTransformerConfig<Message<i32>, i64> = StatefulTransformerConfig::default();
  assert_eq!(config.initial_state, None);
  assert!(config.reset_on_restart);
}

#[test]
fn test_stateful_transformer_config_with_initial_state() {
  let config: StatefulTransformerConfig<Message<i32>, i64> =
    StatefulTransformerConfig::default().with_initial_state(42);
  assert_eq!(config.initial_state, Some(42));
}

#[test]
fn test_stateful_transformer_config_with_reset_on_restart() {
  let config: StatefulTransformerConfig<Message<i32>, i64> =
    StatefulTransformerConfig::default().with_reset_on_restart(false);
  assert!(!config.reset_on_restart);
}

#[test]
fn test_stateful_transformer_config_with_error_strategy() {
  let strategy = ErrorStrategy::Retry(3);
  let config: StatefulTransformerConfig<Message<i32>, i64> =
    StatefulTransformerConfig::default().with_error_strategy(strategy.clone());
  assert_eq!(config.base.error_strategy(), strategy);
}

#[test]
fn test_stateful_transformer_config_with_name() {
  let config: StatefulTransformerConfig<Message<i32>, i64> =
    StatefulTransformerConfig::default().with_name("test_transformer".to_string());
  assert_eq!(config.base.name(), Some("test_transformer".to_string()));
}

#[test]
fn test_stateful_transformer_config_chaining() {
  let config: StatefulTransformerConfig<Message<i32>, i64> = StatefulTransformerConfig::default()
    .with_initial_state(100)
    .with_reset_on_restart(false)
    .with_name("chained".to_string());
  assert_eq!(config.initial_state, Some(100));
  assert!(!config.reset_on_restart);
  assert_eq!(config.base.name(), Some("chained".to_string()));
}

// ============================================================================
// StatefulTransformer Trait Tests
// ============================================================================

#[test]
fn test_stateful_transformer_state() {
  let transformer = MockStatefulTransformer::new(42);
  assert_eq!(transformer.state().unwrap(), Some(42));
}

#[test]
fn test_stateful_transformer_state_or_initial() {
  let transformer = MockStatefulTransformer::new(42);
  assert_eq!(transformer.state_or_initial().unwrap(), 42);
}

#[test]
fn test_stateful_transformer_state_or_initial_empty() {
  // Create transformer with empty state
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let transformer = MockStatefulTransformer {
    state: StdArc::new(store),
    config: TransformerConfig::default(),
  };
  // state_or_initial should fail when both are None
  transformer.state_or_initial().unwrap_err();
}

#[test]
fn test_stateful_transformer_update_state() {
  let transformer = MockStatefulTransformer::new(0);
  let new_state = transformer
    .update_state(|current| current.unwrap_or(0) + 10)
    .unwrap();
  assert_eq!(new_state, 10);
  assert_eq!(transformer.state().unwrap(), Some(10));
}

#[test]
fn test_stateful_transformer_set_state() {
  let transformer = MockStatefulTransformer::new(0);
  transformer.set_state(100).unwrap();
  assert_eq!(transformer.state().unwrap(), Some(100));
}

#[test]
fn test_stateful_transformer_reset_state() {
  let transformer = MockStatefulTransformer::new(42);
  transformer.set_state(100).unwrap();
  assert_eq!(transformer.state().unwrap(), Some(100));

  transformer.reset_state().unwrap();
  assert_eq!(transformer.state().unwrap(), Some(42));
}

#[test]
fn test_stateful_transformer_has_state() {
  let transformer = MockStatefulTransformer::new(42);
  assert!(transformer.has_state());

  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let transformer_without_state = MockStatefulTransformer {
    state: StdArc::new(store),
    config: TransformerConfig::default(),
  };
  assert!(!transformer_without_state.has_state());
}

#[tokio::test]
async fn test_stateful_transformer_integration() {
  let mut transformer = MockStatefulTransformer::new(0);
  let messages: Vec<Message<i32>> = vec![
    Message::new(10, MessageId::new_uuid()),
    Message::new(20, MessageId::new_uuid()),
    Message::new(30, MessageId::new_uuid()),
  ];
  let stream = tokio_stream::iter(messages);

  let output: Vec<Message<i64>> = transformer
    .transform(Box::pin(stream))
    .await
    .collect()
    .await;

  assert_eq!(output.len(), 3);
  assert_eq!(*output[0].payload(), 10);
  assert_eq!(*output[1].payload(), 30);
  assert_eq!(*output[2].payload(), 60);

  // State should be 60 after processing all messages
  assert_eq!(transformer.state().unwrap(), Some(60));
}

// ============================================================================
// StatefulCheckpointError Tests
// ============================================================================

#[test]
fn test_stateful_checkpoint_error_display() {
  let err = StatefulCheckpointError::NoState;
  assert_eq!(format!("{}", err), "No state to checkpoint");

  let err = StatefulCheckpointError::SerializationFailed("json error".to_string());
  assert_eq!(
    format!("{}", err),
    "Checkpoint serialization failed: json error"
  );

  let err = StatefulCheckpointError::DeserializationFailed("parse error".to_string());
  assert_eq!(
    format!("{}", err),
    "Checkpoint deserialization failed: parse error"
  );

  let err = StatefulCheckpointError::IoError(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "file not found",
  ));
  assert!(format!("{}", err).contains("Checkpoint I/O error"));

  let err = StatefulCheckpointError::StateError(StateError::NotInitialized);
  assert!(format!("{}", err).contains("State error during checkpoint"));
}

#[test]
fn test_stateful_checkpoint_error_source() {
  let err = StatefulCheckpointError::IoError(std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "file not found",
  ));
  assert!(err.source().is_some());

  let err = StatefulCheckpointError::NoState;
  assert!(err.source().is_none());
}

#[test]
fn test_stateful_checkpoint_error_from_io_error() {
  let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
  let checkpoint_err: StatefulCheckpointError = io_err.into();
  assert!(matches!(
    checkpoint_err,
    StatefulCheckpointError::IoError(_)
  ));
}

#[test]
fn test_stateful_checkpoint_error_from_state_error() {
  let state_err = StateError::NotInitialized;
  let checkpoint_err: StatefulCheckpointError = state_err.into();
  assert!(matches!(
    checkpoint_err,
    StatefulCheckpointError::StateError(_)
  ));
}

// ============================================================================
// StatefulCheckpointConfig Tests
// ============================================================================

#[test]
fn test_stateful_checkpoint_config_default() {
  let config = StatefulCheckpointConfig::default();
  assert_eq!(config.checkpoint_interval, 0);
  assert!(config.checkpoint_on_complete);
  assert!(config.restore_on_startup);
}

#[test]
fn test_stateful_checkpoint_config_with_interval() {
  let config = StatefulCheckpointConfig::with_interval(100);
  assert_eq!(config.checkpoint_interval, 100);
  assert!(config.checkpoint_on_complete);
  assert!(config.restore_on_startup);
}

#[test]
fn test_stateful_checkpoint_config_checkpoint_on_complete() {
  let config = StatefulCheckpointConfig::default().checkpoint_on_complete(false);
  assert!(!config.checkpoint_on_complete);
}

#[test]
fn test_stateful_checkpoint_config_restore_on_startup() {
  let config = StatefulCheckpointConfig::default().restore_on_startup(false);
  assert!(!config.restore_on_startup);
}

#[test]
fn test_stateful_checkpoint_config_is_auto_checkpoint_enabled() {
  let config = StatefulCheckpointConfig::default();
  assert!(!config.is_auto_checkpoint_enabled());

  let config = StatefulCheckpointConfig::with_interval(10);
  assert!(config.is_auto_checkpoint_enabled());
}

#[test]
fn test_stateful_checkpoint_config_chaining() {
  let config = StatefulCheckpointConfig::default()
    .checkpoint_on_complete(false)
    .restore_on_startup(false);
  assert!(!config.checkpoint_on_complete);
  assert!(!config.restore_on_startup);
}

// ============================================================================
// FileStatefulCheckpointStore Tests
// ============================================================================

#[test]
fn test_file_stateful_checkpoint_store_new() {
  let path = std::path::PathBuf::from("/tmp/test_checkpoint.json");
  let store = FileStatefulCheckpointStore::new(path.clone());
  assert_eq!(store.path(), &path);
}

#[test]
fn test_file_stateful_checkpoint_store_save_load() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("test_checkpoint_save_load.json");
  let store = FileStatefulCheckpointStore::new(path.clone());

  // Clean up any existing file
  let _ = std::fs::remove_file(&path);

  let data = b"test checkpoint data";
  store.save(data).unwrap();

  assert!(store.exists());
  let loaded = store.load().unwrap();
  assert_eq!(loaded, Some(data.to_vec()));

  // Clean up
  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_file_stateful_checkpoint_store_load_nonexistent() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("nonexistent_checkpoint.json");
  let store = FileStatefulCheckpointStore::new(path.clone());

  // Ensure file doesn't exist
  let _ = std::fs::remove_file(&path);

  assert!(!store.exists());
  let loaded = store.load().unwrap();
  assert_eq!(loaded, None);
}

#[test]
fn test_file_stateful_checkpoint_store_clear() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("test_checkpoint_clear.json");
  let store = FileStatefulCheckpointStore::new(path.clone());

  // Create a checkpoint
  store.save(b"test data").unwrap();
  assert!(store.exists());

  // Clear it
  store.clear().unwrap();
  assert!(!store.exists());
}

#[test]
fn test_file_stateful_checkpoint_store_atomic_write() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("test_checkpoint_atomic.json");
  let store = FileStatefulCheckpointStore::new(path.clone());

  // Clean up any existing file
  let _ = std::fs::remove_file(&path);

  let data = b"atomic write test";
  store.save(data).unwrap();

  // Verify the temp file doesn't exist (should have been renamed)
  let temp_path = path.with_extension("tmp");
  assert!(!temp_path.exists());

  // Clean up
  let _ = std::fs::remove_file(&path);
}

// ============================================================================
// InMemoryStatefulCheckpointStore Tests
// ============================================================================

#[test]
fn test_in_memory_stateful_checkpoint_store_new() {
  let store = InMemoryStatefulCheckpointStore::new();
  assert!(!store.exists());
}

#[test]
fn test_in_memory_stateful_checkpoint_store_default() {
  let store: InMemoryStatefulCheckpointStore = InMemoryStatefulCheckpointStore::default();
  assert!(!store.exists());
}

#[test]
fn test_in_memory_stateful_checkpoint_store_save_load() {
  let store = InMemoryStatefulCheckpointStore::new();

  let data = b"test checkpoint data";
  store.save(data).unwrap();

  assert!(store.exists());
  let loaded = store.load().unwrap();
  assert_eq!(loaded, Some(data.to_vec()));
}

#[test]
fn test_in_memory_stateful_checkpoint_store_load_nonexistent() {
  let store = InMemoryStatefulCheckpointStore::new();
  assert!(!store.exists());

  let loaded = store.load().unwrap();
  assert_eq!(loaded, None);
}

#[test]
fn test_in_memory_stateful_checkpoint_store_clear() {
  let store = InMemoryStatefulCheckpointStore::new();

  store.save(b"test data").unwrap();
  assert!(store.exists());

  store.clear().unwrap();
  assert!(!store.exists());

  let loaded = store.load().unwrap();
  assert_eq!(loaded, None);
}

// ============================================================================
// StatefulCheckpointableStateStore Tests
// ============================================================================

#[test]
fn test_stateful_checkpointable_state_store_create_json_checkpoint() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let checkpoint = store.create_json_checkpoint().unwrap();

  let restored: i64 = serde_json::from_slice(&checkpoint).unwrap();
  assert_eq!(restored, 42);
}

#[test]
fn test_stateful_checkpointable_state_store_create_json_checkpoint_no_state() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let result = store.create_json_checkpoint();
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    StatefulCheckpointError::NoState
  ));
}

#[test]
fn test_stateful_checkpointable_state_store_restore_from_json_checkpoint() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let data = serde_json::to_vec(&42i64).unwrap();

  store.restore_from_json_checkpoint(&data).unwrap();
  assert_eq!(store.get().unwrap(), Some(42));
}

#[test]
fn test_stateful_checkpointable_state_store_create_json_checkpoint_pretty() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let checkpoint = store.create_json_checkpoint_pretty().unwrap();

  let restored: i64 = serde_json::from_slice(&checkpoint).unwrap();
  assert_eq!(restored, 42);

  // Pretty JSON should be valid JSON
  let checkpoint_str = String::from_utf8(checkpoint).unwrap();
  // For a single integer, pretty JSON might not have newlines, so just verify it's valid JSON
  let _parsed: serde_json::Value = serde_json::from_str(&checkpoint_str).unwrap();
}

#[test]
fn test_stateful_checkpointable_state_store_save_checkpoint() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let checkpoint_store = InMemoryStatefulCheckpointStore::new();

  store
    .save_checkpoint(&checkpoint_store as &dyn StatefulCheckpointStore)
    .unwrap();

  let loaded = checkpoint_store.load().unwrap();
  assert!(loaded.is_some());
  let restored: i64 = serde_json::from_slice(&loaded.unwrap()).unwrap();
  assert_eq!(restored, 42);
}

#[test]
fn test_stateful_checkpointable_state_store_load_checkpoint() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let checkpoint_store = InMemoryStatefulCheckpointStore::new();

  // Save initial state
  let initial_data = serde_json::to_vec(&100i64).unwrap();
  checkpoint_store.save(&initial_data).unwrap();

  // Load into store
  let loaded = store
    .load_checkpoint(&checkpoint_store as &dyn StatefulCheckpointStore)
    .unwrap();
  assert!(loaded);
  assert_eq!(store.get().unwrap(), Some(100));
}

#[test]
fn test_stateful_checkpointable_state_store_load_checkpoint_nonexistent() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let checkpoint_store = InMemoryStatefulCheckpointStore::new();

  let loaded = store
    .load_checkpoint(&checkpoint_store as &dyn StatefulCheckpointStore)
    .unwrap();
  assert!(!loaded);
  assert_eq!(store.get().unwrap(), None);
}

// ============================================================================
// StatefulCheckpointManager Tests
// ============================================================================

#[test]
fn test_stateful_checkpoint_manager_new() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  assert_eq!(manager.config().checkpoint_interval, 0);
}

#[test]
fn test_stateful_checkpoint_manager_with_file() {
  let temp_dir = std::env::temp_dir();
  let path = temp_dir.join("test_manager_checkpoint.json");
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::with_file(path.clone(), config);

  assert_eq!(manager.config().checkpoint_interval, 0);

  // Clean up
  let _ = std::fs::remove_file(&path);
}

#[test]
fn test_stateful_checkpoint_manager_record_item() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::with_interval(5);
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  // First 4 items should not trigger checkpoint
  for _ in 0..4 {
    assert!(!manager.record_item());
  }

  // 5th item should trigger checkpoint
  assert!(manager.record_item());
}

#[test]
fn test_stateful_checkpoint_manager_record_item_disabled() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default(); // interval = 0
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  assert!(!manager.record_item());
}

#[test]
fn test_stateful_checkpoint_manager_reset_counter() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::with_interval(3);
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  // Trigger checkpoint
  manager.record_item();
  manager.record_item();
  assert!(manager.record_item());

  // Reset counter
  manager.reset_counter();

  // Counter should be reset
  assert!(!manager.record_item());
  assert!(!manager.record_item());
  assert!(manager.record_item());
}

#[test]
fn test_stateful_checkpoint_manager_save() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store).unwrap();

  // Verify checkpoint was saved
  assert!(manager.has_checkpoint());
}

#[test]
fn test_stateful_checkpoint_manager_load() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  // Save initial checkpoint
  let state_store1: InMemoryStateStore<i64> = InMemoryStateStore::new(100);
  manager.save(&state_store1).unwrap();

  // Load into new store
  let state_store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = manager.load(&state_store2).unwrap();
  assert!(loaded);
  assert_eq!(state_store2.get().unwrap(), Some(100));
}

#[test]
fn test_stateful_checkpoint_manager_load_nonexistent() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = manager.load(&state_store).unwrap();
  assert!(!loaded);
}

#[test]
fn test_stateful_checkpoint_manager_clear() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store).unwrap();
  assert!(manager.has_checkpoint());

  manager.clear().unwrap();
  assert!(!manager.has_checkpoint());
}

#[test]
fn test_stateful_checkpoint_manager_has_checkpoint() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  assert!(!manager.has_checkpoint());

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store).unwrap();
  assert!(manager.has_checkpoint());
}

#[test]
fn test_stateful_checkpoint_manager_maybe_checkpoint() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::with_interval(3);
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);

  // First two items should not checkpoint
  // Note: maybe_checkpoint calls record_item() internally, so don't call it separately
  manager.maybe_checkpoint(&state_store).unwrap();
  assert!(!manager.has_checkpoint());

  manager.maybe_checkpoint(&state_store).unwrap();
  assert!(!manager.has_checkpoint());

  // Third item should trigger checkpoint
  manager.maybe_checkpoint(&state_store).unwrap();
  assert!(manager.has_checkpoint());
}

#[test]
fn test_stateful_checkpoint_manager_debug() {
  let store = Box::new(InMemoryStatefulCheckpointStore::new());
  let config = StatefulCheckpointConfig::default();
  let manager = StatefulCheckpointManager::<i64>::new(store, config);

  let debug_str = format!("{:?}", manager);
  assert!(debug_str.contains("StatefulCheckpointManager"));
  assert!(debug_str.contains("config"));
}
