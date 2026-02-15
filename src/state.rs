//! Exactly-once state contract for stateful nodes.
//!
//! This module defines the contract for state backends that support exactly-once
//! updates: key/version semantics and idempotent put. See
//! [docs/exactly-once-state.md](../docs/exactly-once-state.md).

use std::hash::Hash;
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
