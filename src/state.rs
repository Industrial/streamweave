//! Exactly-once state contract for stateful nodes.
//!
//! This module defines the contract for state backends that support exactly-once
//! updates: key/version semantics and idempotent put. See
//! [docs/exactly-once-state.md](../docs/exactly-once-state.md).

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use thiserror::Error;

/// Error type for state backend operations.
#[derive(Error, Debug)]
pub enum StateError {
    /// Serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Storage or I/O error.
    #[error("storage error: {0}")]
    Storage(String),
    /// Other backend-specific error.
    #[error("state error: {0}")]
    Other(String),
}

/// Contract for state backends that support exactly-once updates.
///
/// **Key/version semantics:**
/// - **Key:** The partition of state (e.g. user_id, session_id). Updates are key-scoped.
/// - **Version:** A monotonic identifier (e.g. [`LogicalTime`]). Applying the same
///   `(key, value, version)` multiple times is **idempotent** (no double-apply).
/// - **Ordering:** Updates for the same key should be processed in version order.
///   If an older version arrives after a newer one, the backend may ignore it or
///   apply a defined policy (e.g. last-writer-wins).
///
/// **Idempotency:** `put(key, value, version)` must be idempotent: calling it again
/// with the same arguments must not change the result. Typically implemented as
/// "apply only if version > current_version(key)".
///
/// **Checkpoint/recovery:** `snapshot` and `restore` enable checkpointing; on replay
/// after restore, duplicate (key, value, version) tuples are deduplicated by the backend.
pub trait ExactlyOnceStateBackend {
    /// Key type; must be hashable and comparable for partitioning.
    type Key: Hash + Eq + Clone + Send;
    /// Value type stored per key.
    type Value: Send;
    /// Version type; must be ordered (e.g. `LogicalTime` or `u64`).
    type Version: Ord + Clone + Send;

    /// Put value for key at version. Idempotent: applying the same (key, value, version)
    /// again must not change state. Typically: apply only if version > current version.
    fn put(
        &self,
        key: Self::Key,
        value: Self::Value,
        version: Self::Version,
    ) -> Result<(), StateError>;

    /// Get current value and version for key, or None if not present.
    fn get(&self, key: &Self::Key) -> Result<Option<(Self::Value, Self::Version)>, StateError>;

    /// Snapshot state for checkpoint (serialized form).
    fn snapshot(&self) -> Result<Vec<u8>, StateError>;

    /// Restore state from checkpoint. Overwrites current state.
    fn restore(&mut self, data: &[u8]) -> Result<(), StateError>;
}

/// Snapshot format: serialized list of (key, value, version) tuples.
type SnapshotEntry<K, V, Ver> = (K, (V, Ver));

/// In-memory state backend with put/get/snapshot/restore.
///
/// Implements exactly-once semantics: `put` is idempotent and applies only when
/// `version > current_version(key)`. Snapshot and restore use JSON serialization.
///
/// # Type parameters
///
/// - `K`: Key type; must be serializable
/// - `V`: Value type; must be serializable and cloneable
/// - `Ver`: Version type (e.g. [`crate::time::LogicalTime`] or `u64`)
pub struct HashMapStateBackend<K, V, Ver>
where
    K: Hash + Eq + Clone + Send,
    V: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + Send,
    Ver: Ord + Clone + Send,
{
    inner: Mutex<HashMap<K, (V, Ver)>>,
}

impl<K, V, Ver> Default for HashMapStateBackend<K, V, Ver>
where
    K: Hash + Eq + Clone + Send,
    V: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + Send,
    Ver: Ord + Clone + Send,
{
    fn default() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl<K, V, Ver> HashMapStateBackend<K, V, Ver>
where
    K: Hash + Eq + Clone + Send,
    V: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + Send,
    Ver: Ord + Clone + Send,
{
    /// Creates a new empty in-memory state backend.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, V, Ver> ExactlyOnceStateBackend for HashMapStateBackend<K, V, Ver>
where
    K: Hash + Eq + Clone + Send + serde::Serialize + for<'de> serde::Deserialize<'de>,
    V: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + Send,
    Ver: Ord + Clone + Send + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    type Key = K;
    type Value = V;
    type Version = Ver;

    fn put(
        &self,
        key: Self::Key,
        value: Self::Value,
        version: Self::Version,
    ) -> Result<(), StateError> {
        let mut map = self
            .inner
            .lock()
            .map_err(|e| StateError::Other(e.to_string()))?;
        let should_apply = map
            .get(&key)
            .map_or(true, |(_, v)| version > *v);
        if should_apply {
            map.insert(key, (value, version));
        }
        Ok(())
    }

    fn get(&self, key: &Self::Key) -> Result<Option<(Self::Value, Self::Version)>, StateError> {
        let map = self
            .inner
            .lock()
            .map_err(|e| StateError::Other(e.to_string()))?;
        Ok(map.get(key).cloned())
    }

    fn snapshot(&self) -> Result<Vec<u8>, StateError> {
        let map = self
            .inner
            .lock()
            .map_err(|e| StateError::Other(e.to_string()))?;
        let entries: Vec<SnapshotEntry<K, V, Ver>> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        serde_json::to_vec(&entries).map_err(|e| StateError::Serialization(e.to_string()))
    }

    fn restore(&mut self, data: &[u8]) -> Result<(), StateError> {
        let entries: Vec<SnapshotEntry<K, V, Ver>> =
            serde_json::from_slice(data).map_err(|e| StateError::Serialization(e.to_string()))?;
        let mut map = self
            .inner
            .lock()
            .map_err(|e| StateError::Other(e.to_string()))?;
        map.clear();
        for (key, (value, version)) in entries {
            map.insert(key, (value, version));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::LogicalTime;

    #[test]
    fn hash_map_state_backend_put_get_idempotent() {
        let backend: HashMapStateBackend<String, u64, LogicalTime> = HashMapStateBackend::new();

        backend
            .put("k1".to_string(), 100u64, LogicalTime::new(1))
            .unwrap();
        assert_eq!(
            backend.get(&"k1".to_string()).unwrap(),
            Some((100u64, LogicalTime::new(1)))
        );

        // Same version again: idempotent (we apply only if version > current)
        backend
            .put("k1".to_string(), 999u64, LogicalTime::new(1))
            .unwrap();
        assert_eq!(
            backend.get(&"k1".to_string()).unwrap(),
            Some((100u64, LogicalTime::new(1)))
        );

        // Newer version: applies
        backend
            .put("k1".to_string(), 200u64, LogicalTime::new(2))
            .unwrap();
        assert_eq!(
            backend.get(&"k1".to_string()).unwrap(),
            Some((200u64, LogicalTime::new(2)))
        );
    }

    #[test]
    fn hash_map_state_backend_snapshot_restore() {
        let backend: HashMapStateBackend<String, u64, LogicalTime> = HashMapStateBackend::new();
        backend.put("a".to_string(), 1u64, LogicalTime::new(1)).unwrap();
        backend.put("b".to_string(), 2u64, LogicalTime::new(2)).unwrap();

        let data = backend.snapshot().unwrap();
        assert!(!data.is_empty());

        let mut restored: HashMapStateBackend<String, u64, LogicalTime> = HashMapStateBackend::new();
        restored.restore(&data).unwrap();
        assert_eq!(restored.get(&"a".to_string()).unwrap(), Some((1u64, LogicalTime::new(1))));
        assert_eq!(restored.get(&"b".to_string()).unwrap(), Some((2u64, LogicalTime::new(2))));
    }
}
