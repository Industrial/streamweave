//! Cluster health aggregation for sharded deployments.
//!
//! Combines per-shard readiness into cluster-level health: "all shards running"
//! and "quorum reached". Use when running N workers; the coordinator (or a
//! separate health aggregator) fetches readiness from each worker and calls
//! [`aggregate_cluster_health`].
//!
//! See [production-cluster-tooling.md](../docs/production-cluster-tooling.md) §4.

/// Aggregate cluster health from per-shard readiness.
///
/// Given a slice of readiness flags (one per shard, in shard_id order),
/// returns whether all shards are healthy and whether quorum is reached.
#[derive(Clone, Debug)]
pub struct ClusterHealthReport {
    /// Number of shards that reported ready.
    pub healthy_count: u32,
    /// Total number of shards (expected workers).
    pub total: u32,
    /// True if all shards are ready.
    pub all_healthy: bool,
    /// True if at least a quorum (majority) of shards are ready.
    /// Quorum = ceil(total/2); e.g. 2 of 3, 3 of 5.
    pub quorum_healthy: bool,
}

impl ClusterHealthReport {
    /// Returns true if the cluster is ready to accept work (quorum or all).
    ///
    /// Use for a cluster-level readiness endpoint: return 200 when
    /// `quorum_healthy`, 503 otherwise.
    pub fn is_ready(&self) -> bool {
        self.quorum_healthy
    }
}

/// Aggregates per-shard readiness into a cluster health report.
///
/// Pass readiness in shard order (index 0 = shard 0, etc.). The coordinator
/// or health aggregator should fetch `/ready` (or equivalent) from each worker
/// and collect the results.
///
/// # Example
///
/// ```rust
/// use streamweave::cluster_health::aggregate_cluster_health;
///
/// // 3 workers: shards 0 and 1 ready, shard 2 not ready
/// let readiness = vec![true, true, false];
/// let report = aggregate_cluster_health(&readiness);
/// assert_eq!(report.healthy_count, 2);
/// assert_eq!(report.total, 3);
/// assert!(!report.all_healthy);
/// assert!(report.quorum_healthy);  // 2 >= ceil(3/2)
/// ```
pub fn aggregate_cluster_health(readiness: &[bool]) -> ClusterHealthReport {
    let total = readiness.len() as u32;
    let healthy_count = readiness.iter().filter(|&&r| r).count() as u32;
    let quorum = (total + 1) / 2; // ceil(total/2)
    ClusterHealthReport {
        healthy_count,
        total,
        all_healthy: healthy_count == total && total > 0,
        quorum_healthy: healthy_count >= quorum && total > 0,
    }
}
