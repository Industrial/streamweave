//! Tests for the rebalance protocol.

use crate::partitioning::PartitionKey;
use crate::rebalance::{
    compute_migration_plan, InMemoryCoordinator, RebalanceCoordinator, ShardAssignment,
};

#[tokio::test]
async fn test_shard_assignment_owns_key() {
    let a = ShardAssignment::new(0, 2);
    // Consistency: shard_for_key is deterministic for the same key
    assert_eq!(a.shard_for_key("k0"), a.shard_for_key("k0"));
    assert_eq!(a.owns_key("k0"), a.shard_for_key("k0") == 0);
    // With 1 shard, we own everything
    let single = ShardAssignment::new(0, 1);
    assert!(single.owns_key("any_key"));
}

#[tokio::test]
async fn test_compute_migration_plan_no_change() {
    let keys: Vec<PartitionKey> = ["a", "b", "c"].iter().map(|s| PartitionKey::from(*s)).collect();
    let plan = compute_migration_plan(0, 2, 2, Some(&keys));
    assert!(plan.is_empty());
    assert!(plan.keys_to_export.is_empty());
    assert!(plan.keys_to_import.is_empty());
}

#[tokio::test]
async fn test_compute_migration_plan_add_worker() {
    // With 2 shards: shard 0 owns keys where hash % 2 == 0
    // With 3 shards: shard 0 owns keys where hash % 3 == 0
    // Some keys move from 0 to others, some move to 0 from others
    let keys: Vec<PartitionKey> = (0..20)
        .map(|i| PartitionKey::from(i.to_string()))
        .collect();
    let plan = compute_migration_plan(0, 2, 3, Some(&keys));
    // Plan should partition keys: export = owned under 2 shards but not 3; import = not owned under 2 but owned under 3
    for k in &plan.keys_to_export {
        assert!(!plan.keys_to_import.iter().any(|ki| ki.as_str() == k.as_str()));
    }
}

#[tokio::test]
async fn test_compute_migration_plan_remove_worker() {
    let keys: Vec<PartitionKey> = (0..20)
        .map(|i| PartitionKey::from(i.to_string()))
        .collect();
    let plan = compute_migration_plan(0, 3, 2, Some(&keys));
    assert!(!plan.keys_to_export.is_empty() || !plan.keys_to_import.is_empty());
}

#[tokio::test]
async fn test_in_memory_coordinator() {
    let coord = InMemoryCoordinator::new(0, 2);
    let a = coord.current_assignment().await;
    assert_eq!(a.shard_id, 0);
    assert_eq!(a.total_shards, 2);

    coord.add_worker();
    let a = coord.current_assignment().await;
    assert_eq!(a.total_shards, 3);

    coord.remove_worker();
    let a = coord.current_assignment().await;
    assert_eq!(a.total_shards, 2);

    coord.scale_to(5);
    let a = coord.current_assignment().await;
    assert_eq!(a.shard_id, 0);
    assert_eq!(a.total_shards, 5);
}
