//! Checkpointing for state recovery.

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

/// In-memory checkpoint store (for testing).
pub struct InMemoryCheckpointStore {
  checkpoints: tokio::sync::RwLock<HashMap<String, Checkpoint>>,
}

impl InMemoryCheckpointStore {
  /// Creates a new in-memory checkpoint store.
  #[must_use]
  pub fn new() -> Self {
    Self {
      checkpoints: tokio::sync::RwLock::new(HashMap::new()),
    }
  }
}

impl Default for InMemoryCheckpointStore {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait::async_trait]
impl CheckpointStore for InMemoryCheckpointStore {
  async fn save(&self, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
    let mut checkpoints = self.checkpoints.write().await;
    checkpoints.insert(checkpoint.checkpoint_id.clone(), checkpoint.clone());
    Ok(())
  }

  async fn load_latest(
    &self,
    worker_id: &str,
    partition_id: usize,
  ) -> Result<Option<Checkpoint>, CheckpointError> {
    let checkpoints = self.checkpoints.read().await;
    let matching: Vec<&Checkpoint> = checkpoints
      .values()
      .filter(|c| c.worker_id == worker_id && c.partition_id == partition_id)
      .collect();

    // Return the latest checkpoint
    Ok(matching.into_iter().max_by_key(|c| c.timestamp).cloned())
  }

  async fn load(&self, checkpoint_id: &str) -> Result<Checkpoint, CheckpointError> {
    let checkpoints = self.checkpoints.read().await;
    checkpoints
      .get(checkpoint_id)
      .cloned()
      .ok_or_else(|| CheckpointError::NotFound(checkpoint_id.to_string()))
  }

  async fn list(&self, worker_id: &str) -> Result<Vec<Checkpoint>, CheckpointError> {
    let checkpoints = self.checkpoints.read().await;
    Ok(
      checkpoints
        .values()
        .filter(|c| c.worker_id == worker_id)
        .cloned()
        .collect(),
    )
  }

  async fn delete(&self, checkpoint_id: &str) -> Result<(), CheckpointError> {
    let mut checkpoints = self.checkpoints.write().await;
    checkpoints.remove(checkpoint_id);
    Ok(())
  }
}
