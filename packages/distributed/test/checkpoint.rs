use proptest::prelude::*;
use std::collections::HashMap;
use streamweave_distributed::{
  Checkpoint, CheckpointError, CheckpointStore, InMemoryCheckpointStore,
};

async fn test_in_memory_checkpoint_store_async(
  checkpoint_id: String,
  worker_id: String,
  partition_id: usize,
  offset: u64,
  state_data: Vec<u8>,
) {
  let store = InMemoryCheckpointStore::new();
  let checkpoint = Checkpoint {
    checkpoint_id: checkpoint_id.clone(),
    timestamp: chrono::Utc::now(),
    worker_id: worker_id.clone(),
    partition_id,
    offset,
    state_data: state_data.clone(),
    metadata: HashMap::new(),
  };

  store.save(&checkpoint).await.unwrap();
  let loaded = store.load(&checkpoint_id).await.unwrap();
  assert_eq!(loaded.checkpoint_id, checkpoint_id);
  assert_eq!(loaded.worker_id, worker_id);
  assert_eq!(loaded.partition_id, partition_id);
  assert_eq!(loaded.offset, offset);
  assert_eq!(loaded.state_data, state_data);
}

proptest! {
  #[test]
  fn test_in_memory_checkpoint_store(
    checkpoint_id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
    worker_id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
    partition_id in 0usize..1000,
    offset in 0u64..1000000,
    state_data in prop::collection::vec(any::<u8>(), 0..1000),
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_in_memory_checkpoint_store_async(
      checkpoint_id,
      worker_id,
      partition_id,
      offset,
      state_data,
    ));
  }
}

#[tokio::test]
async fn test_checkpoint_store_load_latest() {
  let store = InMemoryCheckpointStore::new();
  let worker_id = "worker-1";
  let partition_id = 0;

  // No checkpoints initially
  let latest = store.load_latest(worker_id, partition_id).await.unwrap();
  assert!(latest.is_none());

  // Save multiple checkpoints
  let checkpoint1 = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now() - chrono::Duration::seconds(10),
    worker_id: worker_id.to_string(),
    partition_id,
    offset: 100,
    state_data: vec![1, 2, 3],
    metadata: HashMap::new(),
  };

  let checkpoint2 = Checkpoint {
    checkpoint_id: "cp2".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: worker_id.to_string(),
    partition_id,
    offset: 200,
    state_data: vec![4, 5, 6],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint1).await.unwrap();
  store.save(&checkpoint2).await.unwrap();

  // Latest should be checkpoint2
  let latest = store.load_latest(worker_id, partition_id).await.unwrap();
  assert!(latest.is_some());
  assert_eq!(latest.unwrap().checkpoint_id, "cp2");
  assert_eq!(latest.unwrap().offset, 200);
}

#[tokio::test]
async fn test_checkpoint_store_load_not_found() {
  let store = InMemoryCheckpointStore::new();
  let result = store.load("nonexistent").await;
  assert!(result.is_err());
  match result.unwrap_err() {
    CheckpointError::NotFound(id) => assert_eq!(id, "nonexistent"),
    _ => panic!("Expected NotFound error"),
  }
}

#[tokio::test]
async fn test_checkpoint_store_list() {
  let store = InMemoryCheckpointStore::new();
  let worker_id = "worker-1";

  // Save checkpoints for different workers
  let checkpoint1 = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: worker_id.to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  let checkpoint2 = Checkpoint {
    checkpoint_id: "cp2".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: worker_id.to_string(),
    partition_id: 1,
    offset: 200,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  let checkpoint3 = Checkpoint {
    checkpoint_id: "cp3".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-2".to_string(),
    partition_id: 0,
    offset: 300,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint1).await.unwrap();
  store.save(&checkpoint2).await.unwrap();
  store.save(&checkpoint3).await.unwrap();

  // List should only return checkpoints for worker-1
  let list = store.list(worker_id).await.unwrap();
  assert_eq!(list.len(), 2);
  assert!(list.iter().all(|c| c.worker_id == worker_id));
}

#[tokio::test]
async fn test_checkpoint_store_delete() {
  let store = InMemoryCheckpointStore::new();
  let checkpoint = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint).await.unwrap();
  assert!(store.load("cp1").await.is_ok());

  store.delete("cp1").await.unwrap();
  assert!(store.load("cp1").await.is_err());
}

#[tokio::test]
async fn test_checkpoint_store_multiple_workers() {
  let store = InMemoryCheckpointStore::new();

  // Save checkpoints for different workers and partitions
  for worker_id in ["worker-1", "worker-2"] {
    for partition_id in 0..3 {
      let checkpoint = Checkpoint {
        checkpoint_id: format!("cp-{}-{}", worker_id, partition_id),
        timestamp: chrono::Utc::now(),
        worker_id: worker_id.to_string(),
        partition_id,
        offset: (partition_id as u64) * 100,
        state_data: vec![],
        metadata: HashMap::new(),
      };
      store.save(&checkpoint).await.unwrap();
    }
  }

  // Verify load_latest returns correct checkpoint for each worker/partition
  let latest = store.load_latest("worker-1", 0).await.unwrap();
  assert!(latest.is_some());
  assert_eq!(latest.unwrap().checkpoint_id, "cp-worker-1-0");

  let latest = store.load_latest("worker-2", 1).await.unwrap();
  assert!(latest.is_some());
  assert_eq!(latest.unwrap().checkpoint_id, "cp-worker-2-1");
}

#[test]
fn test_checkpoint_error_display() {
  let error = CheckpointError::Serialization("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Serialization error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_checkpoint_error_invalid_format() {
  let error = CheckpointError::InvalidFormat("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Invalid checkpoint format"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_checkpoint_error_other() {
  let error = CheckpointError::Other("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Checkpoint error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_checkpoint_error_debug() {
  let error = CheckpointError::NotFound("test".to_string());
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_checkpoint_error_is_error() {
  let error = CheckpointError::NotFound("test".to_string());
  use std::error::Error;
  assert!(!error.source().is_some());
}

#[tokio::test]
async fn test_checkpoint_store_default() {
  let store = InMemoryCheckpointStore::default();
  let checkpoint = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: HashMap::new(),
  };

  store.save(&checkpoint).await.unwrap();
  let loaded = store.load("cp1").await.unwrap();
  assert_eq!(loaded.checkpoint_id, "cp1");
}

#[tokio::test]
async fn test_checkpoint_with_metadata() {
  let store = InMemoryCheckpointStore::new();
  let mut metadata = HashMap::new();
  metadata.insert("key1".to_string(), "value1".to_string());
  metadata.insert("key2".to_string(), "value2".to_string());

  let checkpoint = Checkpoint {
    checkpoint_id: "cp1".to_string(),
    timestamp: chrono::Utc::now(),
    worker_id: "worker-1".to_string(),
    partition_id: 0,
    offset: 100,
    state_data: vec![],
    metadata: metadata.clone(),
  };

  store.save(&checkpoint).await.unwrap();
  let loaded = store.load("cp1").await.unwrap();
  assert_eq!(loaded.metadata, metadata);
}

proptest! {
  #[test]
  fn test_checkpoint_serialization(
    checkpoint_id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
    worker_id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
    partition_id in 0usize..1000,
    offset in 0u64..1000000,
  ) {
    let checkpoint = Checkpoint {
      checkpoint_id: checkpoint_id.clone(),
      timestamp: chrono::Utc::now(),
      worker_id: worker_id.clone(),
      partition_id,
      offset,
      state_data: vec![],
      metadata: HashMap::new(),
    };

    let serialized = serde_json::to_string(&checkpoint).unwrap();
    let deserialized: Checkpoint = serde_json::from_str(&serialized).unwrap();

    prop_assert_eq!(checkpoint.checkpoint_id, deserialized.checkpoint_id);
    prop_assert_eq!(checkpoint.worker_id, deserialized.worker_id);
    prop_assert_eq!(checkpoint.partition_id, deserialized.partition_id);
    prop_assert_eq!(checkpoint.offset, deserialized.offset);
  }
}
