//! Coordinated checkpoint protocol and recovery for distributed workers.
//!
//! Defines the message contract, storage layout, coordinator trait, and
//! recovery plan for barrier-based aligned checkpoints across N shards.
//! See [distributed-checkpointing.md](../../docs/distributed-checkpointing.md) §6–7.

use crate::checkpoint::{CheckpointError, CheckpointId, CheckpointMetadata};
use crate::partitioning::PartitionKey;
use crate::time::LogicalTime;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// Request from coordinator to worker: take checkpoint at optional barrier time.
#[derive(Clone, Debug)]
pub struct CheckpointRequest {
    /// Checkpoint identifier assigned by the coordinator.
    pub checkpoint_id: CheckpointId,
    /// Optional barrier (logical time T) at which to snapshot; None = drain and snapshot.
    pub barrier_t: Option<LogicalTime>,
}

/// Report from worker to coordinator: checkpoint done or failed.
#[derive(Clone, Debug)]
pub struct CheckpointDone {
    /// Checkpoint identifier.
    pub checkpoint_id: CheckpointId,
    /// Shard that completed.
    pub shard_id: u32,
    /// True if snapshot and save succeeded.
    pub success: bool,
}

/// Optional ack from coordinator to worker when checkpoint is committed.
#[derive(Clone, Debug)]
pub struct CheckpointCommitted {
    /// Checkpoint identifier that was committed.
    pub checkpoint_id: CheckpointId,
}

/// Storage backend for distributed checkpoints.
///
/// Workers write to `<base>/<id>/shard_<shard_id>/`. The coordinator (or storage
/// implementation) writes `COMMITTED` when all workers have reported.
pub trait DistributedCheckpointStorage: Send + Sync {
    /// Saves this shard's snapshot to shared storage under `<base>/<id>/shard_<shard_id>/`.
    fn save_shard(
        &self,
        checkpoint_id: CheckpointId,
        shard_id: u32,
        metadata: &CheckpointMetadata,
        snapshots: &HashMap<String, Vec<u8>>,
    ) -> Result<(), CheckpointError>;

    /// Marks the checkpoint as committed (writes COMMITTED flag). Called by coordinator
    /// when all workers have reported. Default: no-op (coordinator may use external store).
    fn mark_committed(&self, checkpoint_id: CheckpointId) -> Result<(), CheckpointError> {
        let _ = checkpoint_id;
        Ok(())
    }

    /// Returns true if the checkpoint is committed. Used by workers to verify before
    /// releasing pre-checkpoint state.
    fn is_committed(&self, checkpoint_id: CheckpointId) -> Result<bool, CheckpointError> {
        let _ = checkpoint_id;
        Ok(false)
    }

    /// Loads this shard's snapshot from a committed checkpoint.
    fn load_shard(
        &self,
        checkpoint_id: CheckpointId,
        shard_id: u32,
    ) -> Result<(CheckpointMetadata, HashMap<String, Vec<u8>>), CheckpointError>;
}

/// File-based distributed checkpoint storage.
///
/// Layout: `<base>/<id>/shard_<shard_id>/metadata.json`, `<node>.bin`;
/// `<base>/<id>/COMMITTED` when committed.
pub struct FileDistributedCheckpointStorage {
    base_path: std::path::PathBuf,
}

impl FileDistributedCheckpointStorage {
    /// Creates storage at the given base path.
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    fn shard_dir(&self, checkpoint_id: CheckpointId, shard_id: u32) -> std::path::PathBuf {
        self.base_path
            .join(checkpoint_id.as_u64().to_string())
            .join(format!("shard_{}", shard_id))
    }

    fn committed_path(&self, checkpoint_id: CheckpointId) -> std::path::PathBuf {
        self.base_path
            .join(checkpoint_id.as_u64().to_string())
            .join("COMMITTED")
    }
}

impl DistributedCheckpointStorage for FileDistributedCheckpointStorage {
    fn save_shard(
        &self,
        checkpoint_id: CheckpointId,
        shard_id: u32,
        metadata: &CheckpointMetadata,
        snapshots: &HashMap<String, Vec<u8>>,
    ) -> Result<(), CheckpointError> {
        let dir = self.shard_dir(checkpoint_id, shard_id);
        std::fs::create_dir_all(&dir)?;

        let metadata_path = dir.join("metadata.json");
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| CheckpointError::Serialization(e.to_string()))?;
        std::fs::write(metadata_path, json)?;

        for (node_id, data) in snapshots {
            let safe_name = node_id
                .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
            let snapshot_path = dir.join(format!("{}.bin", safe_name));
            std::fs::write(snapshot_path, data)?;
        }

        Ok(())
    }

    fn mark_committed(&self, checkpoint_id: CheckpointId) -> Result<(), CheckpointError> {
        let dir = self.base_path.join(checkpoint_id.as_u64().to_string());
        std::fs::create_dir_all(&dir)?;
        std::fs::write(self.committed_path(checkpoint_id), b"")?;
        Ok(())
    }

    fn is_committed(&self, checkpoint_id: CheckpointId) -> Result<bool, CheckpointError> {
        Ok(self.committed_path(checkpoint_id).exists())
    }

    fn load_shard(
        &self,
        checkpoint_id: CheckpointId,
        shard_id: u32,
    ) -> Result<(CheckpointMetadata, HashMap<String, Vec<u8>>), CheckpointError> {
        let dir = self.shard_dir(checkpoint_id, shard_id);
        if !dir.exists() {
            return Err(CheckpointError::NotFound(format!(
                "shard {} for checkpoint {}",
                shard_id,
                checkpoint_id.as_u64()
            )));
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
}

/// Coordinator that triggers checkpoints and collects worker reports.
///
/// Implement this trait to plug in external coordination (e.g. custom service,
/// ZooKeeper, or Kafka). StreamWeave does not provide a built-in distributed
/// coordinator.
#[async_trait]
pub trait CheckpointCoordinator: Send + Sync {
    /// Requests a new checkpoint. Returns the request to send to workers (or assigns id).
    /// Workers call their graph's trigger_checkpoint_for_coordination and report_done.
    async fn request_checkpoint(&self) -> CheckpointRequest;

    /// Called by workers when they complete (success or failure).
    async fn report_done(&self, done: CheckpointDone);

    /// Waits until the checkpoint is committed (all workers reported) or aborted.
    /// Returns true if committed, false if aborted.
    async fn await_commit(&self, checkpoint_id: CheckpointId) -> bool;
}

/// In-memory coordinator for tests and single-process demos.
///
/// Collects reports from workers; commits when all `total_shards` have reported
/// successfully. Aborts if any worker reports failure.
pub struct InMemoryCheckpointCoordinator {
    total_shards: u32,
    next_id: std::sync::atomic::AtomicU64,
    pending: std::sync::RwLock<
        std::collections::HashMap<
            CheckpointId,
            (
                std::sync::Arc<tokio::sync::Notify>,
                std::sync::atomic::AtomicU32,
                std::sync::Arc<std::sync::atomic::AtomicBool>,
            ),
        >,
    >,
}

impl InMemoryCheckpointCoordinator {
    /// Creates a coordinator expecting the given number of shards.
    pub fn new(total_shards: u32) -> Self {
        Self {
            total_shards,
            next_id: std::sync::atomic::AtomicU64::new(1),
            pending: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl CheckpointCoordinator for InMemoryCheckpointCoordinator {
    async fn request_checkpoint(&self) -> CheckpointRequest {
        let id = CheckpointId::new(
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );
        {
            let mut pending = self.pending.write().unwrap();
            pending.insert(
                id,
                (
                    std::sync::Arc::new(tokio::sync::Notify::new()),
                    std::sync::atomic::AtomicU32::new(0),
                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
                ),
            );
        }
        CheckpointRequest {
            checkpoint_id: id,
            barrier_t: None,
        }
    }

    async fn report_done(&self, done: CheckpointDone) {
        let pending = self.pending.read().unwrap();
        if let Some((notify, count, all_ok)) = pending.get(&done.checkpoint_id) {
            if !done.success {
                all_ok.store(false, std::sync::atomic::Ordering::SeqCst);
            }
            let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if n >= self.total_shards {
                notify.notify_waiters();
            }
        }
    }

    async fn await_commit(&self, checkpoint_id: CheckpointId) -> bool {
        let notify_and_ok = {
            let pending = self.pending.read().unwrap();
            pending
                .get(&checkpoint_id)
                .map(|(n, _, ok)| (std::sync::Arc::clone(n), std::sync::Arc::clone(ok)))
        };
        if let Some((notify, all_ok)) = notify_and_ok {
            notify.notified().await;
            all_ok.load(std::sync::atomic::Ordering::SeqCst)
        } else {
            false
        }
    }
}

// -----------------------------------------------------------------------------
// Recovery from worker failure (phase 5)
// -----------------------------------------------------------------------------

/// Recovery strategy when a worker fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryStrategy {
    /// Absorb keys: reduce total_shards; remaining workers absorb the failed shard's keys.
    Absorb,
    /// Replace: start a new worker with the same shard_id; no key migration.
    Replace,
}

/// Plan for one surviving worker during recovery (Option A: absorb).
#[derive(Clone, Debug)]
pub struct RecoveryStep {
    /// This worker's old shard id (before failure).
    pub old_shard_id: u32,
    /// This worker's new shard id (after renumbering).
    pub new_shard_id: u32,
    /// New total number of shards.
    pub new_total_shards: u32,
    /// Keys to import from the failed shard (subset of keys the failed shard owned).
    pub keys_to_import: Vec<PartitionKey>,
}

/// Computes the recovery plan when a worker fails and we use Option A (absorb).
///
/// Surviving workers are renumbered so shard ids remain 0..new_total-1. Returns
/// one `RecoveryStep` per surviving worker. Each step's `keys_to_import` is
/// computed from `keys` (typically the set of keys the failed shard owned); pass
/// `None` to get steps with empty `keys_to_import` (caller supplies keys later).
pub fn compute_recovery_plan_absorb(
    failed_shard_id: u32,
    old_total_shards: u32,
    keys: Option<&[PartitionKey]>,
) -> Vec<RecoveryStep> {
    if old_total_shards <= 1 {
        return Vec::new();
    }
    let new_total = old_total_shards - 1;

    let mut steps = Vec::new();
    for old_id in 0..old_total_shards {
        if old_id == failed_shard_id {
            continue;
        }
        let new_id = if old_id < failed_shard_id {
            old_id
        } else {
            old_id - 1
        };

        let keys_to_import: Vec<PartitionKey> = match keys {
            Some(k) => {
                let failed_owns = |s: &str| {
                    crate::rebalance::ShardAssignment::new(failed_shard_id, old_total_shards)
                        .owns_key(s)
                };
                let new_assignment = crate::rebalance::ShardAssignment::new(new_id, new_total);
                k.iter()
                    .filter(|key| failed_owns(key.as_str()) && new_assignment.owns_key(key.as_str()))
                    .cloned()
                    .collect()
            }
            None => Vec::new(),
        };

        steps.push(RecoveryStep {
            old_shard_id: old_id,
            new_shard_id: new_id,
            new_total_shards: new_total,
            keys_to_import,
        });
    }
    steps
}
