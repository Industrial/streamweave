//! Tests for distributed checkpoint module

use std::collections::HashMap;
use streamweave::distributed::checkpoint::{
  Checkpoint, CheckpointError, CheckpointStore, InMemoryCheckpointStore,
};

#[tokio::test]
async fn test_checkpoint_new() {
  let checkpoint = Checkpoint {
    checkpoint_id: "checkpoint-1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![1, 2, 3],
    metadata: HashMap::new(),
  };

  assert_eq!(checkpoint.checkpoint_id, "checkpoint-1");
  assert_eq!(checkpoint.worker_id, "worker-1");
  assert_eq!(checkpoint.partition_id, 0);
  assert_eq!(checkpoint.offset, 100);
}

#[test]
fn test_checkpoint_error_display() {
  let err = CheckpointError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
  assert!(err.to_string().contains("I/O error"));

  let err = CheckpointError::Serialization("test".to_string());
  assert!(err.to_string().contains("Serialization"));

  let err = CheckpointError::NotFound("test".to_string());
  assert!(err.to_string().contains("Checkpoint not found"));

  let err = CheckpointError::InvalidFormat("test".to_string());
  assert!(err.to_string().contains("Invalid checkpoint format"));

  let err = CheckpointError::Other("test".to_string());
  assert!(err.to_string().contains("Checkpoint error"));
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_new() {
  let store = InMemoryCheckpointStore::new();
  // Should create successfully
  assert!(true);
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_default() {
  let store = InMemoryCheckpointStore::default();
  // Should create successfully
  assert!(true);
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_save_and_load() {
  let store = InMemoryCheckpointStore::new();
  let checkpoint = Checkpoint {
    checkpoint_id: "checkpoint-1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![1, 2, 3],
    metadata: HashMap::new(),
  };

  // Save checkpoint
  store.save(&checkpoint).await.unwrap();

  // Load checkpoint
  let loaded = store.load("checkpoint-1").await.unwrap();
  assert_eq!(loaded.checkpoint_id, "checkpoint-1");
  assert_eq!(loaded.offset, 100);
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_load_not_found() {
  let store = InMemoryCheckpointStore::new();
  let result = store.load("nonexistent").await;
  assert!(matches!(result, Err(CheckpointError::NotFound(_))));
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_load_latest() {
  let store = InMemoryCheckpointStore::new();

  let checkpoint1 = Checkpoint {
    checkpoint_id: "checkpoint-1".to_string(),
    timestamp: chrono::Utc::now() - chrono::Duration::seconds(10),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  let checkpoint2 = Checkpoint {
    checkpoint_id: "checkpoint-2".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 200,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint1).await.unwrap();
  store.save(&checkpoint2).await.unwrap();

  // Should return the latest checkpoint
  let latest = store.load_latest("worker-1", 0).await.unwrap().unwrap();
  assert_eq!(latest.checkpoint_id, "checkpoint-2");
  assert_eq!(latest.offset, 200);
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_list() {
  let store = InMemoryCheckpointStore::new();

  let checkpoint1 = Checkpoint {
    checkpoint_id: "checkpoint-1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  let checkpoint2 = Checkpoint {
    checkpoint_id: "checkpoint-2".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 1,
    offset: 200,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint1).await.unwrap();
  store.save(&checkpoint2).await.unwrap();

  let checkpoints = store.list("worker-1").await.unwrap();
  assert_eq!(checkpoints.len(), 2);
}

#[tokio::test]
async fn test_in_memory_checkpoint_store_delete() {
  let store = InMemoryCheckpointStore::new();
  let checkpoint = Checkpoint {
    checkpoint_id: "checkpoint-1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint).await.unwrap();
  store.delete("checkpoint-1").await.unwrap();

  // Should not exist after deletion
  let result = store.load("checkpoint-1").await;
  assert!(matches!(result, Err(CheckpointError::NotFound(_))));
}
