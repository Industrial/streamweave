//! Tests for StatefulTransformer

use streamweave_stateful::*;

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

// ============================================================================
// Additional tests for comprehensive coverage
// ============================================================================

#[test]
fn test_deserialize_empty_data() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let empty_data = b"";

  // Empty data should set to default
  store.deserialize_and_set_state(empty_data).unwrap();
  assert_eq!(store.get().unwrap(), Some(0)); // i64::default() is 0
}

#[test]
fn test_state_store_ext_update() {
  // Test the StateStoreExt::update convenience method
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(10);

  // This uses the StateStoreExt::update method (blanket impl)
  let result = store.update(|current| current.unwrap_or(0) + 5).unwrap();
  assert_eq!(result, 15);
  assert_eq!(store.get().unwrap(), Some(15));
}

// ============================================================================
// CheckpointError tests
// ============================================================================

#[test]
fn test_checkpoint_error_display() {
  assert_eq!(
    format!("{}", CheckpointError::NoState),
    "No state to checkpoint"
  );
  assert_eq!(
    format!(
      "{}",
      CheckpointError::SerializationFailed("bad".to_string())
    ),
    "Checkpoint serialization failed: bad"
  );
  assert_eq!(
    format!(
      "{}",
      CheckpointError::DeserializationFailed("bad".to_string())
    ),
    "Checkpoint deserialization failed: bad"
  );
  assert_eq!(
    format!(
      "{}",
      CheckpointError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found"
      ))
    ),
    "Checkpoint I/O error: file not found"
  );
  assert_eq!(
    format!(
      "{}",
      CheckpointError::StateError(StateError::NotInitialized)
    ),
    "State error during checkpoint: State is not initialized"
  );
}

#[test]
fn test_checkpoint_error_from_io_error() {
  let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
  let checkpoint_err: CheckpointError = io_err.into();
  assert!(matches!(checkpoint_err, CheckpointError::IoError(_)));
}

#[test]
fn test_checkpoint_error_from_state_error() {
  let state_err = StateError::NotInitialized;
  let checkpoint_err: CheckpointError = state_err.into();
  assert!(matches!(checkpoint_err, CheckpointError::StateError(_)));
}

#[test]
fn test_checkpoint_error_source() {
  let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
  let checkpoint_err = CheckpointError::IoError(io_err);
  assert!(checkpoint_err.source().is_some());
}

// ============================================================================
// CheckpointConfig tests
// ============================================================================

#[test]
fn test_checkpoint_config_default() {
  let config = CheckpointConfig::default();
  assert_eq!(config.checkpoint_interval, 0);
  assert!(config.checkpoint_on_complete);
  assert!(config.restore_on_startup);
  assert!(!config.is_auto_checkpoint_enabled());
}

#[test]
fn test_checkpoint_config_with_interval() {
  let config = CheckpointConfig::with_interval(100);
  assert_eq!(config.checkpoint_interval, 100);
  assert!(config.is_auto_checkpoint_enabled());
}

#[test]
fn test_checkpoint_config_checkpoint_on_complete() {
  let config = CheckpointConfig::default().checkpoint_on_complete(false);
  assert!(!config.checkpoint_on_complete);
}

#[test]
fn test_checkpoint_config_restore_on_startup() {
  let config = CheckpointConfig::default().restore_on_startup(false);
  assert!(!config.restore_on_startup);
}

// ============================================================================
// FileCheckpointStore tests
// ============================================================================

#[test]
fn test_file_checkpoint_store_new() {
  use std::path::PathBuf;
  let path = PathBuf::from("/tmp/test_checkpoint.json");
  let store = FileCheckpointStore::new(path.clone());
  assert_eq!(store.path(), &path);
}

#[test]
fn test_file_checkpoint_store_save_and_load() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("checkpoint.json");
  let store = FileCheckpointStore::new(path.clone());

  // Save checkpoint
  let data = b"test checkpoint data";
  store.save(data).unwrap();

  // Load checkpoint
  let loaded = store.load().unwrap();
  assert_eq!(loaded, Some(data.to_vec()));
}

#[test]
fn test_file_checkpoint_store_load_nonexistent() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("nonexistent.json");
  let store = FileCheckpointStore::new(path);

  let loaded = store.load().unwrap();
  assert_eq!(loaded, None);
}

#[test]
fn test_file_checkpoint_store_clear() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("checkpoint.json");
  let store = FileCheckpointStore::new(path.clone());

  // Save and verify exists
  store.save(b"data").unwrap();
  assert!(store.exists());

  // Clear and verify doesn't exist
  store.clear().unwrap();
  assert!(!store.exists());
}

#[test]
fn test_file_checkpoint_store_exists() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("checkpoint.json");
  let store = FileCheckpointStore::new(path.clone());

  // Initially doesn't exist
  assert!(!store.exists());

  // After save, exists
  store.save(b"data").unwrap();
  assert!(store.exists());
}

#[test]
fn test_file_checkpoint_store_save_creates_parent_dirs() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("subdir").join("checkpoint.json");
  let store = FileCheckpointStore::new(path.clone());

  // Save should create parent directory
  store.save(b"data").unwrap();
  assert!(store.exists());
  assert!(path.parent().unwrap().exists());
}

// ============================================================================
// InMemoryCheckpointStore tests
// ============================================================================

#[test]
fn test_in_memory_checkpoint_store_new() {
  let store = InMemoryCheckpointStore::new();
  assert!(!store.exists());
}

#[test]
fn test_in_memory_checkpoint_store_default() {
  let store = InMemoryCheckpointStore::default();
  assert!(!store.exists());
}

#[test]
fn test_in_memory_checkpoint_store_save_and_load() {
  let store = InMemoryCheckpointStore::new();
  let data = b"test data";

  store.save(data).unwrap();
  assert!(store.exists());

  let loaded = store.load().unwrap();
  assert_eq!(loaded, Some(data.to_vec()));
}

#[test]
fn test_in_memory_checkpoint_store_load_nonexistent() {
  let store = InMemoryCheckpointStore::new();
  let loaded = store.load().unwrap();
  assert_eq!(loaded, None);
}

#[test]
fn test_in_memory_checkpoint_store_clear() {
  let store = InMemoryCheckpointStore::new();
  store.save(b"data").unwrap();
  assert!(store.exists());

  store.clear().unwrap();
  assert!(!store.exists());
}

#[test]
fn test_in_memory_checkpoint_store_exists() {
  let store = InMemoryCheckpointStore::new();
  assert!(!store.exists());

  store.save(b"data").unwrap();
  assert!(store.exists());
}

// ============================================================================
// CheckpointableStateStore tests
// ============================================================================

#[test]
fn test_create_json_checkpoint() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let checkpoint = store.create_json_checkpoint().unwrap();
  assert!(!checkpoint.is_empty());

  let value: i64 = serde_json::from_slice(&checkpoint).unwrap();
  assert_eq!(value, 42);
}

#[test]
fn test_create_json_checkpoint_no_state() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let result = store.create_json_checkpoint();
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), CheckpointError::NoState));
}

#[test]
fn test_restore_from_json_checkpoint() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let data = serde_json::to_vec(&100i64).unwrap();

  store.restore_from_json_checkpoint(&data).unwrap();
  assert_eq!(store.get().unwrap(), Some(100));
}

#[test]
fn test_restore_from_json_checkpoint_invalid() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let invalid_data = b"not valid json";

  let result = store.restore_from_json_checkpoint(invalid_data);
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    CheckpointError::DeserializationFailed(_)
  ));
}

#[test]
fn test_create_json_checkpoint_pretty() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  let checkpoint = store.create_json_checkpoint_pretty().unwrap();
  assert!(!checkpoint.is_empty());

  // Pretty JSON should contain newlines
  assert!(checkpoint.contains(&b'\n'));

  let value: i64 = serde_json::from_slice(&checkpoint).unwrap();
  assert_eq!(value, 42);
}

#[test]
fn test_save_checkpoint() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("checkpoint.json");
  let file_store = FileCheckpointStore::new(path);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  state_store.save_checkpoint(&file_store).unwrap();

  // Verify checkpoint was saved
  assert!(file_store.exists());
  let loaded = file_store.load().unwrap();
  assert!(loaded.is_some());
  let value: i64 = serde_json::from_slice(&loaded.unwrap()).unwrap();
  assert_eq!(value, 42);
}

#[test]
fn test_load_checkpoint() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("checkpoint.json");
  let file_store = FileCheckpointStore::new(path);

  // Save checkpoint first
  let data = serde_json::to_vec(&100i64).unwrap();
  file_store.save(&data).unwrap();

  // Load into state store
  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = state_store.load_checkpoint(&file_store).unwrap();
  assert!(loaded);
  assert_eq!(state_store.get().unwrap(), Some(100));
}

#[test]
fn test_load_checkpoint_nonexistent() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("nonexistent.json");
  let file_store = FileCheckpointStore::new(path);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = state_store.load_checkpoint(&file_store).unwrap();
  assert!(!loaded);
}

// ============================================================================
// CheckpointManager tests
// ============================================================================

#[test]
fn test_checkpoint_manager_new() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);
  assert!(!manager.has_checkpoint());
}

#[test]
fn test_checkpoint_manager_with_file() {
  use tempfile::TempDir;
  let temp_dir = TempDir::new().unwrap();
  let path = temp_dir.path().join("checkpoint.json");
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::with_file(path, config);
  assert!(!manager.has_checkpoint());
}

#[test]
fn test_checkpoint_manager_config() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::with_interval(100);
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config.clone());
  assert_eq!(manager.config().checkpoint_interval, 100);
}

#[test]
fn test_checkpoint_manager_record_item() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::with_interval(5);
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  // Record items up to interval
  for i in 1..=5 {
    let should_checkpoint = manager.record_item();
    if i == 5 {
      assert!(should_checkpoint);
    } else {
      assert!(!should_checkpoint);
    }
  }
}

#[test]
fn test_checkpoint_manager_record_item_disabled() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default(); // interval = 0
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  // Should never trigger checkpoint when disabled
  assert!(!manager.record_item());
}

#[test]
fn test_checkpoint_manager_reset_counter() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::with_interval(5);
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  // Record 5 items to trigger checkpoint
  for _ in 0..5 {
    manager.record_item();
  }

  // Reset counter
  manager.reset_counter();

  // Next item should not trigger checkpoint
  assert!(!manager.record_item());
}

#[test]
fn test_checkpoint_manager_save() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store).unwrap();

  assert!(manager.has_checkpoint());
  manager.reset_counter();
}

#[test]
fn test_checkpoint_manager_load() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  // Save first
  let state_store1: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store1).unwrap();

  // Load into new store
  let state_store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = manager.load(&state_store2).unwrap();
  assert!(loaded);
  assert_eq!(state_store2.get().unwrap(), Some(42));
}

#[test]
fn test_checkpoint_manager_load_nonexistent() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = manager.load(&state_store).unwrap();
  assert!(!loaded);
}

#[test]
fn test_checkpoint_manager_clear() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  // Save checkpoint
  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store).unwrap();
  assert!(manager.has_checkpoint());

  // Clear checkpoint
  manager.clear().unwrap();
  assert!(!manager.has_checkpoint());
}

#[test]
fn test_checkpoint_manager_has_checkpoint() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  assert!(!manager.has_checkpoint());

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  manager.save(&state_store).unwrap();
  assert!(manager.has_checkpoint());
}

#[test]
fn test_checkpoint_manager_maybe_checkpoint() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::with_interval(3);
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);

  // Record items - should checkpoint on 3rd
  for i in 1..=3 {
    manager.maybe_checkpoint(&state_store).unwrap();
    if i == 3 {
      assert!(manager.has_checkpoint());
    }
  }
}

#[test]
fn test_checkpoint_manager_maybe_checkpoint_disabled() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default(); // interval = 0
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  let state_store: InMemoryStateStore<i64> = InMemoryStateStore::new(0);

  // Should not checkpoint when disabled
  manager.maybe_checkpoint(&state_store).unwrap();
  assert!(!manager.has_checkpoint());
}

#[test]
fn test_checkpoint_manager_debug() {
  let store = Box::new(InMemoryCheckpointStore::new());
  let config = CheckpointConfig::default();
  let manager: CheckpointManager<i64> = CheckpointManager::new(store, config);

  // Just verify Debug is implemented
  let _ = format!("{:?}", manager);
}

// ============================================================================
// StatefulTransformer trait tests (using a mock implementation)
// ============================================================================

// Mock StatefulTransformer implementation for testing
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig};

struct MockStatefulTransformer {
  state: InMemoryStateStore<i64>,
  config: TransformerConfig<i32>,
}

impl MockStatefulTransformer {
  fn new() -> Self {
    Self {
      state: InMemoryStateStore::new(0),
      config: TransformerConfig::default(),
    }
  }
}

impl Input for MockStatefulTransformer {
  type Input = i32;
  type InputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

impl Output for MockStatefulTransformer {
  type Output = i32;
  type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

#[async_trait]
impl Transformer for MockStatefulTransformer {
  type InputPorts = (i32,);
  type OutputPorts = (i32,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    use futures::StreamExt;
    Box::pin(input.map(|x| x))
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
    &mut self.state
  }
}

#[tokio::test]
async fn test_stateful_transformer_state() {
  let transformer = MockStatefulTransformer::new();
  let state = transformer.state().unwrap();
  assert_eq!(state, Some(0));
}

#[tokio::test]
async fn test_stateful_transformer_state_or_initial() {
  let transformer = MockStatefulTransformer::new();
  let state = transformer.state_or_initial().unwrap();
  assert_eq!(state, 0);
}

#[tokio::test]
async fn test_stateful_transformer_state_or_initial_no_state() {
  let mut transformer = MockStatefulTransformer::new();
  transformer.state_store_mut().set(42).unwrap();
  transformer.state_store_mut().reset().unwrap();
  transformer.state_store_mut().set(100).unwrap(); // Set to something
  transformer.state_store_mut().reset().unwrap(); // Reset again

  // If state is None and initial is Some(0), should return 0
  let state = transformer.state_or_initial().unwrap();
  assert_eq!(state, 0);
}

#[tokio::test]
async fn test_stateful_transformer_update_state() {
  let transformer = MockStatefulTransformer::new();
  let new_state = transformer
    .update_state(|current| current.unwrap_or(0) + 10)
    .unwrap();
  assert_eq!(new_state, 10);
  assert_eq!(transformer.state().unwrap(), Some(10));
}

#[tokio::test]
async fn test_stateful_transformer_set_state() {
  let transformer = MockStatefulTransformer::new();
  transformer.set_state(42).unwrap();
  assert_eq!(transformer.state().unwrap(), Some(42));
}

#[tokio::test]
async fn test_stateful_transformer_reset_state() {
  let mut transformer = MockStatefulTransformer::new();
  transformer.set_state(100).unwrap();
  transformer.reset_state().unwrap();
  assert_eq!(transformer.state().unwrap(), Some(0)); // Back to initial
}

#[tokio::test]
async fn test_stateful_transformer_has_state() {
  let transformer = MockStatefulTransformer::new();
  assert!(transformer.has_state());

  let mut transformer = MockStatefulTransformer::new();
  transformer.state_store_mut().reset().unwrap();
  assert!(!transformer.has_state());
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[test]
fn test_stateful_transformer_config_with_error_strategy() {
  use streamweave::error::ErrorStrategy;
  let config: StatefulTransformerConfig<i32, i64> =
    StatefulTransformerConfig::default().with_error_strategy(ErrorStrategy::Skip);
  // Just verify it compiles and doesn't panic
  assert!(config.base.error_strategy().is_some());
}

#[test]
fn test_in_memory_state_store_update_with() {
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(10);
  let result = store
    .update_with(Box::new(|current| current.unwrap_or(0) + 5))
    .unwrap();
  assert_eq!(result, 15);
}

#[test]
fn test_state_error_error_trait() {
  let err: Box<dyn std::error::Error> = Box::new(StateError::NotInitialized);
  assert!(err.source().is_none());
}
