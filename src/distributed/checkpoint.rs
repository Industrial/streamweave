//! Checkpointing for state recovery.

use chrono;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Checkpoint metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
  /// Checkpoint identifier.
  pub checkpoint_id: String,
  /// Timestamp when checkpoint was created.
  pub timestamp: chrono::DateTime<chrono::Utc>,
  /// Worker ID that created the checkpoint.
  pub worker_id: String,
  /// Partition ID this checkpoint is for.
  pub partition_id: usize,
  /// Offset/position in the stream.
  pub offset: u64,
  /// Serialized state data.
  pub state_data: Vec<u8>,
  /// Metadata (key-value pairs).
  pub metadata: HashMap<String, String>,
}

/// Checkpoint-related errors.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
  /// I/O error during checkpoint operation.
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),

  /// Serialization error.
  #[error("Serialization error: {0}")]
  Serialization(String),

  /// Checkpoint not found.
  #[error("Checkpoint not found: {0}")]
  NotFound(String),

  /// Invalid checkpoint format.
  #[error("Invalid checkpoint format: {0}")]
  InvalidFormat(String),

  /// Other checkpoint error.
  #[error("Checkpoint error: {0}")]
  Other(String),
}

/// Trait for storing and retrieving checkpoints.
#[async_trait::async_trait]
pub trait CheckpointStore: Send + Sync {
  /// Saves a checkpoint.
  async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError>;

  /// Loads the latest checkpoint for a worker and partition.
  async fn load_latest(
    &self,
    worker_id: &str,
    partition_id: usize,
  ) -> Result<Option<Checkpoint>, CheckpointError>;

  /// Loads a specific checkpoint by ID.
  async fn load(&self, checkpoint_id: &str) -> Result<Checkpoint, CheckpointError>;

  /// Lists all checkpoints for a worker.
  async fn list(&self, worker_id: &str) -> Result<Vec<Checkpoint>, CheckpointError>;

  /// Deletes a checkpoint.
  async fn delete(&self, checkpoint_id: &str) -> Result<(), CheckpointError>;
}

pub struct InMemoryCheckpointStore {
  checkpoints: tokio::sync::Mutex<HashMap<String, Checkpoint>>,
}

impl Default for InMemoryCheckpointStore {
  fn default() -> Self {
    Self::new()
  }
}

impl InMemoryCheckpointStore {
  pub fn new() -> Self {
    Self {
      checkpoints: tokio::sync::Mutex::new(HashMap::new()),
    }
  }
}

#[async_trait::async_trait]
impl CheckpointStore for InMemoryCheckpointStore {
  async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
    let mut checkpoints = self.checkpoints.lock().await;
    checkpoints.insert(checkpoint.checkpoint_id.clone(), checkpoint.clone());
    Ok(())
  }

  async fn load_latest(
    &self,
    worker_id: &str,
    partition_id: usize,
  ) -> Result<Option<Checkpoint>, CheckpointError> {
    let checkpoints = self.checkpoints.lock().await;
    let latest = checkpoints
      .values()
      .filter(|c| c.worker_id == worker_id && c.partition_id == partition_id)
      .max_by_key(|c| c.timestamp)
      .cloned();
    Ok(latest)
  }

  async fn load(&self, checkpoint_id: &str) -> Result<Checkpoint, CheckpointError> {
    let checkpoints = self.checkpoints.lock().await;
    checkpoints
      .get(checkpoint_id)
      .cloned()
      .ok_or_else(|| CheckpointError::NotFound(checkpoint_id.to_string()))
  }

  async fn list(&self, worker_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
    let checkpoints = self.checkpoints.lock().await;
    let mut worker_checkpoints: Vec<Checkpoint> = checkpoints
      .values()
      .filter(|c| c.worker_id == worker_id)
      .cloned()
      .collect();
    worker_checkpoints.sort_by_key(|c| c.timestamp);
    Ok(worker_checkpoints)
  }

  async fn delete(&self, checkpoint_id: &str) -> Result<(), CheckpointError> {
    let mut checkpoints = self.checkpoints.lock().await;
    if checkpoints.remove(checkpoint_id).is_some() {
      Ok(())
    } else {
      Err(CheckpointError::NotFound(checkpoint_id.to_string()))
    }
  }
}
