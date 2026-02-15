//! Local and coordinated checkpointing for stateful nodes.
//!
//! Persists state snapshots and optional positions to durable storage for
//! recovery. See [docs/distributed-checkpointing.md](../../docs/distributed-checkpointing.md).

mod coordinated;
#[cfg(test)]
mod coordinated_test;

pub use coordinated::{
    CheckpointCommitted, CheckpointCoordinator, CheckpointDone, CheckpointRequest,
    DistributedCheckpointStorage, FileDistributedCheckpointStorage,
    InMemoryCheckpointCoordinator,
};

use crate::time::LogicalTime;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Error type for checkpoint operations.
#[derive(Error, Debug)]
pub enum CheckpointError {
    /// I/O or filesystem error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Checkpoint not found.
    #[error("checkpoint not found: {0}")]
    NotFound(String),
    /// Other error.
    #[error("checkpoint error: {0}")]
    Other(String),
}

/// Identifier for a checkpoint (e.g. logical time or sequence number).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CheckpointId(pub u64);

impl CheckpointId {
    /// Creates a new checkpoint id from a raw value.
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the raw u64 value.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<LogicalTime> for CheckpointId {
    fn from(t: LogicalTime) -> Self {
        Self(t.as_u64())
    }
}

/// Metadata for a checkpoint.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CheckpointMetadata {
    /// Checkpoint identifier.
    pub id: CheckpointId,
    /// Optional position (e.g. logical time up to which input was consumed).
    pub position: Option<LogicalTime>,
}

/// Trait for checkpoint storage backends.
pub trait CheckpointStorage: Send + Sync {
    /// Saves a checkpoint with the given metadata and per-node snapshots.
    fn save(
        &self,
        metadata: &CheckpointMetadata,
        snapshots: &HashMap<String, Vec<u8>>,
    ) -> Result<(), CheckpointError>;

    /// Loads a checkpoint by id. Returns metadata and per-node snapshots.
    fn load(&self, id: CheckpointId) -> Result<(CheckpointMetadata, HashMap<String, Vec<u8>>), CheckpointError>;

    /// Lists available checkpoint ids.
    fn list(&self) -> Result<Vec<CheckpointId>, CheckpointError>;
}

/// File-based checkpoint storage.
///
/// Writes checkpoints to a directory. Each checkpoint is stored as a subdirectory
/// `<base>/<id>/` containing `metadata.json` and per-node snapshot files
/// `<node_id>.bin`.
pub struct FileCheckpointStorage {
    base_path: std::path::PathBuf,
}

impl FileCheckpointStorage {
    /// Creates a new file checkpoint storage at the given path.
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    fn checkpoint_dir(&self, id: CheckpointId) -> std::path::PathBuf {
        self.base_path.join(id.as_u64().to_string())
    }
}

impl CheckpointStorage for FileCheckpointStorage {
    fn save(
        &self,
        metadata: &CheckpointMetadata,
        snapshots: &HashMap<String, Vec<u8>>,
    ) -> Result<(), CheckpointError> {
        let dir = self.checkpoint_dir(metadata.id);
        std::fs::create_dir_all(&dir)?;

        let metadata_path = dir.join("metadata.json");
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;
        std::fs::write(metadata_path, json)?;

        for (node_id, data) in snapshots {
            let safe_name = node_id.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
            let snapshot_path = dir.join(format!("{}.bin", safe_name));
            std::fs::write(snapshot_path, data)?;
        }

        Ok(())
    }

    fn load(&self, id: CheckpointId) -> Result<(CheckpointMetadata, HashMap<String, Vec<u8>>), CheckpointError> {
        let dir = self.checkpoint_dir(id);
        if !dir.exists() {
            return Err(CheckpointError::NotFound(id.as_u64().to_string()));
        }

        let metadata_path = dir.join("metadata.json");
        let json = std::fs::read_to_string(&metadata_path)?;
        let metadata: CheckpointMetadata =
            serde_json::from_str(&json).map_err(|e| CheckpointError::Serialization(e.to_string()))?;

        let mut snapshots = HashMap::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "bin") {
                let node_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let data = std::fs::read(&path)?;
                snapshots.insert(node_id, data);
            }
        }

        Ok((metadata, snapshots))
    }

    fn list(&self) -> Result<Vec<CheckpointId>, CheckpointError> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let name = entry.file_name();
            if let Some(s) = name.to_str() {
                if let Ok(id) = s.parse::<u64>() {
                    ids.push(CheckpointId::new(id));
                }
            }
        }
        ids.sort_by_key(|c| c.0);
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn file_checkpoint_storage_save_load() {
        let tmp = TempDir::new().unwrap();
        let storage = FileCheckpointStorage::new(tmp.path());

        let metadata = CheckpointMetadata {
            id: CheckpointId::new(1),
            position: Some(LogicalTime::new(100)),
        };
        let mut snapshots = HashMap::new();
        snapshots.insert("node_a".to_string(), vec![1, 2, 3]);
        snapshots.insert("node_b".to_string(), vec![4, 5, 6]);

        storage.save(&metadata, &snapshots).unwrap();
        let (loaded_meta, loaded_snapshots) = storage.load(CheckpointId::new(1)).unwrap();
        assert_eq!(loaded_meta.id.0, 1);
        assert_eq!(loaded_meta.position, Some(LogicalTime::new(100)));
        assert_eq!(loaded_snapshots.get("node_a"), Some(&vec![1, 2, 3]));
        assert_eq!(loaded_snapshots.get("node_b"), Some(&vec![4, 5, 6]));

        let ids = storage.list().unwrap();
        assert_eq!(ids, vec![CheckpointId::new(1)]);
    }
}
