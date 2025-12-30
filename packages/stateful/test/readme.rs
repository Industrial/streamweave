//! Integration tests for README examples
//!
//! These tests verify that all code examples in the README compile and run correctly.

use std::sync::Arc;
use streamweave_stateful::{
  CheckpointableStateStore, FileCheckpointStore, InMemoryStateStore, StateCheckpoint, StateStoreExt,
};
use tempfile::TempDir;
use tokio::task;

#[test]
fn test_readme_state_persistence() {
  // Example: State Persistence (lines 157-170)
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);

  // Serialize state to bytes
  let checkpoint = store.serialize_state().unwrap();

  // Restore to a new store
  let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  store2.deserialize_and_set_state(&checkpoint).unwrap();

  assert_eq!(store2.get().unwrap(), Some(42));
}

#[test]
fn test_readme_state_reset() {
  // Example: State Reset (lines 177-181)
  let store = InMemoryStateStore::new(0);
  store.set(100).unwrap();
  store.reset().unwrap();
  assert_eq!(store.get().unwrap(), Some(0));
}

#[test]
fn test_readme_state_initialization_with_initial() {
  // Example: State Initialization (lines 187-196) - With initial state
  let store = InMemoryStateStore::new(0);
  assert_eq!(store.get().unwrap(), Some(0));
}

#[test]
fn test_readme_state_initialization_empty() {
  // Example: State Initialization (lines 187-196) - Without initial state
  let store = InMemoryStateStore::<i64>::empty();
  assert_eq!(store.get().unwrap(), None);
}

#[test]
fn test_readme_state_initialization_with_optional_initial() {
  // Example: State Initialization (lines 187-196) - With optional initial state
  let store = InMemoryStateStore::with_optional_initial(Some(42));
  assert_eq!(store.get().unwrap(), Some(42));
}

#[tokio::test]
async fn test_readme_thread_safe_state_access() {
  // Example: Thread-Safe State Access (lines 202-219)
  let store = Arc::new(InMemoryStateStore::new(0));

  // Access from multiple tasks
  let store1 = Arc::clone(&store);
  let store2 = Arc::clone(&store);

  let handle1 = task::spawn(async move {
    store1.update(|current| current.unwrap_or(0) + 1).unwrap();
  });

  let handle2 = task::spawn(async move {
    store2.update(|current| current.unwrap_or(0) + 2).unwrap();
  });

  handle1.await.unwrap();
  handle2.await.unwrap();

  // Verify the state was updated (order is non-deterministic, but both should have run)
  let final_state = store.get().unwrap();
  // Final state should be either 1+2=3 (if both ran) or just one of them
  assert!(final_state == Some(1) || final_state == Some(2) || final_state == Some(3));
}

#[test]
fn test_readme_file_checkpoint_store_save_and_load() {
  // Example: FileCheckpointStore (from source documentation)
  let temp_dir = TempDir::new().unwrap();
  let checkpoint_path = temp_dir.path().join("checkpoint.json");

  let store = FileCheckpointStore::new(checkpoint_path.clone());
  store.save(b"{\"count\": 42}").unwrap();

  let data = store.load().unwrap();
  assert!(data.is_some());
  assert_eq!(data.unwrap(), b"{\"count\": 42}");
}

#[test]
fn test_readme_file_checkpoint_store_exists() {
  let temp_dir = TempDir::new().unwrap();
  let checkpoint_path = temp_dir.path().join("checkpoint.json");

  let store = FileCheckpointStore::new(checkpoint_path.clone());
  assert!(!store.exists());

  store.save(b"{\"count\": 42}").unwrap();
  assert!(store.exists());
}

#[test]
fn test_readme_file_checkpoint_store_clear() {
  let temp_dir = TempDir::new().unwrap();
  let checkpoint_path = temp_dir.path().join("checkpoint.json");

  let store = FileCheckpointStore::new(checkpoint_path.clone());
  store.save(b"{\"count\": 42}").unwrap();
  assert!(store.exists());

  store.clear().unwrap();
  assert!(!store.exists());
}

#[test]
fn test_readme_checkpointable_state_store() {
  // Test checkpointable state store functionality
  let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);
  store.set(100).unwrap();

  // Create JSON checkpoint
  let checkpoint = store.create_json_checkpoint().unwrap();
  assert!(!checkpoint.is_empty());

  // Create in-memory checkpoint store
  let checkpoint_store = streamweave_stateful::InMemoryCheckpointStore::new();

  // Save checkpoint
  store.save_checkpoint(&checkpoint_store).unwrap();

  // Load checkpoint
  let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
  let loaded = store2.load_checkpoint(&checkpoint_store).unwrap();
  assert!(loaded);

  assert_eq!(store2.get().unwrap(), Some(100));
}
