//! Rebalance protocol for cluster sharding.
//!
//! Defines types and coordination for adding/removing workers and migrating
//! state. Keys are partitioned by `hash(key) % total_shards`; when
//! `total_shards` changes, a migration plan describes which keys each shard
//! must export or import.
//!
//! See [cluster-sharding.md](../docs/cluster-sharding.md) for the full protocol.
//!
//! ## API for external controllers (auto-scaling)
//!
//! External controllers (e.g. Kubernetes HPA, custom scaler) can drive rebalance:
//!
//! - **Get assignment:** [`RebalanceCoordinator::current_assignment`]
//! - **Scale (InMemoryCoordinator):** [`InMemoryCoordinator::scale_to`] sets
//!   `total_shards`; workers poll or are notified and run drain/migrate/resume.
//! - **Production:** Implement `RebalanceCoordinator` with etcd, ZooKeeper, or
//!   Kafka consumer group; the controller updates the shared store to trigger
//!   assignment changes.

use crate::partitioning::PartitionKey;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Assignment of a shard: (shard_id, total_shards). Keys map via hash % total_shards.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShardAssignment {
  /// This shard's index (0..total_shards-1).
  pub shard_id: u32,
  /// Total number of shards in the cluster.
  pub total_shards: u32,
}

impl ShardAssignment {
  /// Creates a new assignment.
  pub fn new(shard_id: u32, total_shards: u32) -> Self {
    Self {
      shard_id,
      total_shards,
    }
  }

  /// Returns the shard index that owns the given key under this assignment.
  pub fn shard_for_key(&self, key: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let h = hasher.finish();
    (h % self.total_shards as u64) as u32
  }

  /// Returns true if this shard owns the key.
  pub fn owns_key(&self, key: &str) -> bool {
    self.shard_for_key(key) == self.shard_id
  }
}

/// Plan for one shard during rebalance: which keys to export and which to import.
#[derive(Clone, Debug, Default)]
pub struct MigrationPlan {
  /// Keys this shard must export (state sent to gaining shard).
  pub keys_to_export: Vec<PartitionKey>,
  /// Keys this shard must import (state received from losing shard).
  pub keys_to_import: Vec<PartitionKey>,
}

impl MigrationPlan {
  /// Returns true if this shard has no migration work.
  pub fn is_empty(&self) -> bool {
    self.keys_to_export.is_empty() && self.keys_to_import.is_empty()
  }
}

/// Computes the migration plan for a shard when the cluster changes size.
///
/// Given (shard_id, old_total, new_total) and an optional list of keys to consider,
/// returns keys this shard loses (export) and gains (import). If `keys` is None,
/// the plan describes the abstract set; callers must supply the actual key list
/// when exporting (e.g. from a state backend).
///
/// For a full migration, the caller typically iterates over all known keys and
/// filters by owns_key under old vs new assignment.
pub fn compute_migration_plan(
  shard_id: u32,
  old_total: u32,
  new_total: u32,
  keys: Option<&[PartitionKey]>,
) -> MigrationPlan {
  if old_total == new_total {
    return MigrationPlan::default();
  }

  let old_assignment = ShardAssignment::new(shard_id, old_total);
  let new_assignment = ShardAssignment::new(shard_id, new_total);

  let (mut keys_to_export, mut keys_to_import) = (Vec::new(), Vec::new());

  // If no keys provided, return empty plan (caller will use state backend to enumerate).
  let keys: Vec<PartitionKey> = match keys {
    Some(k) => k.to_vec(),
    None => return MigrationPlan::default(),
  };

  for key in keys {
    let key_str = key.as_str();
    let owned_old = old_assignment.owns_key(key_str);
    let owned_new = new_assignment.owns_key(key_str);
    if owned_old && !owned_new {
      keys_to_export.push(key);
    } else if !owned_old && owned_new {
      keys_to_import.push(key);
    }
  }

  MigrationPlan {
    keys_to_export,
    keys_to_import,
  }
}

/// Event describing a cluster change that triggers rebalance.
#[derive(Clone, Debug)]
pub enum RebalanceEvent {
  /// A new worker joined; total_shards increased.
  WorkerJoined {
    /// New total number of shards after the worker joined.
    new_total_shards: u32,
  },
  /// A worker left; total_shards decreased.
  WorkerLeft {
    /// New total number of shards after the worker left.
    new_total_shards: u32,
  },
  /// Assignment changed (e.g. external coordinator updated assignment).
  AssignmentChanged {
    /// The new shard assignment for this worker.
    new_assignment: ShardAssignment,
  },
}

/// Coordinator that decides shard assignment and notifies workers of changes.
///
/// Implement this trait to plug in external coordination (e.g. Kafka consumer
/// group, ZooKeeper, or a custom leader). StreamWeave does not provide a
/// built-in distributed coordinator; use this trait with your chosen system.
#[async_trait::async_trait]
pub trait RebalanceCoordinator: Send + Sync {
  /// Returns the current assignment for this worker.
  async fn current_assignment(&self) -> ShardAssignment;

  /// Waits for the next assignment change (e.g. worker join/leave).
  ///
  /// Returns None if the coordinator is shut down.
  async fn await_assignment_change(&self) -> Option<ShardAssignment>;
}

/// In-memory coordinator for tests and single-process demos.
///
/// Holds a fixed assignment. Use [`InMemoryCoordinator::add_worker`] and
/// [`InMemoryCoordinator::remove_worker`] to simulate cluster changes.
pub struct InMemoryCoordinator {
  inner: std::sync::RwLock<(u32, u32)>,
}

impl std::fmt::Debug for InMemoryCoordinator {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let (sid, tot) = *self.inner.read().unwrap();
    f.debug_struct("InMemoryCoordinator")
      .field("shard_id", &sid)
      .field("total_shards", &tot)
      .finish()
  }
}

impl InMemoryCoordinator {
  /// Creates a coordinator with the given initial assignment.
  pub fn new(shard_id: u32, total_shards: u32) -> Self {
    Self {
      inner: std::sync::RwLock::new((shard_id, total_shards)),
    }
  }

  /// Simulates adding a worker (total_shards += 1).
  pub fn add_worker(&self) {
    let mut g = self.inner.write().unwrap();
    g.1 += 1;
  }

  /// Simulates removing a worker (total_shards -= 1).
  ///
  /// Panics if total_shards would drop below 1.
  pub fn remove_worker(&self) {
    let mut g = self.inner.write().unwrap();
    assert!(g.1 > 1, "cannot remove last worker");
    g.1 -= 1;
  }

  /// Sets the cluster to the given number of shards (for external controller API).
  ///
  /// Keeps this worker's shard_id; updates total_shards. Use when an external
  /// controller (e.g. Kubernetes HPA) decides to scale the cluster. Workers
  /// should then run the rebalance protocol (drain, migrate, resume).
  pub fn scale_to(&self, total_shards: u32) {
    assert!(total_shards >= 1, "total_shards must be >= 1");
    let mut g = self.inner.write().unwrap();
    g.1 = total_shards;
  }

  /// Updates the assignment directly.
  pub fn set_assignment(&self, shard_id: u32, total_shards: u32) {
    let mut g = self.inner.write().unwrap();
    *g = (shard_id, total_shards);
  }
}

#[async_trait::async_trait]
impl RebalanceCoordinator for InMemoryCoordinator {
  async fn current_assignment(&self) -> ShardAssignment {
    let (shard_id, total_shards) = *self.inner.read().unwrap();
    ShardAssignment::new(shard_id, total_shards)
  }

  async fn await_assignment_change(&self) -> Option<ShardAssignment> {
    // In-memory coordinator has no async notification; returns None.
    // Tests drive changes via add_worker/remove_worker and poll current_assignment.
    None
  }
}
