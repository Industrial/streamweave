//! Tests for coordinated checkpoint protocol.

use crate::checkpoint::{
    CheckpointCoordinator, CheckpointDone, CheckpointRequest, DistributedCheckpointStorage,
    FileDistributedCheckpointStorage, InMemoryCheckpointCoordinator,
};

#[tokio::test]
async fn test_chandy_lamport_request() {
    let req = CheckpointRequest::chandy_lamport(CheckpointId::new(1));
    assert_eq!(req.checkpoint_id.as_u64(), 1);
    assert_eq!(req.barrier_t, None);
    assert_eq!(req.mode, crate::checkpoint::CheckpointMode::ChandyLamport);
}
use crate::checkpoint::{CheckpointId, CheckpointMetadata};
use crate::graph::Graph;
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_in_memory_coordinator_commit() {
    let coord = InMemoryCheckpointCoordinator::new(2);
    let req = coord.request_checkpoint().await;
    assert_eq!(req.checkpoint_id.as_u64(), 1);

    coord
        .report_done(CheckpointDone {
            checkpoint_id: req.checkpoint_id,
            shard_id: 0,
            success: true,
        })
        .await;
    coord
        .report_done(CheckpointDone {
            checkpoint_id: req.checkpoint_id,
            shard_id: 1,
            success: true,
        })
        .await;

    let committed = coord.await_commit(req.checkpoint_id).await;
    assert!(committed);
}

#[tokio::test]
async fn test_in_memory_coordinator_abort_on_failure() {
    let coord = InMemoryCheckpointCoordinator::new(2);
    let req = coord.request_checkpoint().await;

    coord
        .report_done(CheckpointDone {
            checkpoint_id: req.checkpoint_id,
            shard_id: 0,
            success: false,
        })
        .await;
    coord
        .report_done(CheckpointDone {
            checkpoint_id: req.checkpoint_id,
            shard_id: 1,
            success: true,
        })
        .await;

    let committed = coord.await_commit(req.checkpoint_id).await;
    assert!(!committed);
}

#[tokio::test]
async fn test_file_distributed_storage_save_load_shard() {
    let tmp = TempDir::new().unwrap();
    let storage = FileDistributedCheckpointStorage::new(tmp.path());

    let metadata = CheckpointMetadata {
        id: CheckpointId::new(1),
        position: None,
    };
    let mut snapshots = HashMap::new();
    snapshots.insert("node_a".to_string(), vec![1, 2, 3]);

    storage
        .save_shard(CheckpointId::new(1), 0, &metadata, &snapshots)
        .unwrap();

    let (loaded_meta, loaded_snapshots) = storage.load_shard(CheckpointId::new(1), 0).unwrap();
    assert_eq!(loaded_meta.id.as_u64(), 1);
    assert_eq!(loaded_snapshots.get("node_a"), Some(&vec![1, 2, 3]));

    storage.mark_committed(CheckpointId::new(1)).unwrap();
    assert!(storage.is_committed(CheckpointId::new(1)).unwrap());
}

#[tokio::test]
async fn test_graph_trigger_checkpoint_for_coordination() {
    let tmp = TempDir::new().unwrap();
    let storage = FileDistributedCheckpointStorage::new(tmp.path());

    let mut graph = Graph::new("g".to_string());
    graph
        .add_node(
            "stateful".to_string(),
            Box::new(MockSnapshotNode::new(
                "stateful",
                vec![1, 2, 3],
            )),
        )
        .unwrap();

    let req = CheckpointRequest::barrier(CheckpointId::new(42), None);

    let done = graph
        .trigger_checkpoint_for_coordination(&storage, &req, 0)
        .unwrap();

    assert_eq!(done.checkpoint_id.as_u64(), 42);
    assert_eq!(done.shard_id, 0);
    assert!(done.success);

    let (meta, snapshots) = storage.load_shard(CheckpointId::new(42), 0).unwrap();
    assert_eq!(meta.id.as_u64(), 42);
    assert_eq!(snapshots.get("stateful"), Some(&vec![1, 2, 3]));
}

/// Minimal node that returns non-empty snapshot for testing.
struct MockSnapshotNode {
    name: String,
    snapshot_data: Vec<u8>,
}

impl MockSnapshotNode {
    fn new(name: &str, data: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            snapshot_data: data,
        }
    }
}

#[async_trait::async_trait]
impl crate::node::Node for MockSnapshotNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    fn input_port_names(&self) -> &[String] {
        &[]
    }
    fn output_port_names(&self) -> &[String] {
        static PORTS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["out".to_string()]);
        &PORTS
    }
    fn has_input_port(&self, _name: &str) -> bool {
        false
    }
    fn has_output_port(&self, name: &str) -> bool {
        name == "out"
    }
    fn execute(
        &self,
        _inputs: crate::node::InputStreams,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<crate::node::OutputStreams, crate::node::NodeExecutionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { Ok(std::collections::HashMap::new()) })
    }
    fn snapshot_state(&self) -> Result<Vec<u8>, crate::node::NodeExecutionError> {
        Ok(self.snapshot_data.clone())
    }
}

#[test]
fn test_compute_recovery_plan_absorb() {
    use crate::checkpoint::compute_recovery_plan_absorb;
    use crate::partitioning::PartitionKey;

    let steps = compute_recovery_plan_absorb(1, 3, None);
    assert_eq!(steps.len(), 2); // shards 0 and 2 survive
    assert_eq!(steps[0].old_shard_id, 0);
    assert_eq!(steps[0].new_shard_id, 0);
    assert_eq!(steps[0].new_total_shards, 2);
    assert_eq!(steps[1].old_shard_id, 2);
    assert_eq!(steps[1].new_shard_id, 1);
    assert_eq!(steps[1].new_total_shards, 2);

    let keys: Vec<PartitionKey> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|s| PartitionKey::from(*s))
        .collect();
    let steps = compute_recovery_plan_absorb(0, 2, Some(&keys));
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].new_shard_id, 0);
    assert_eq!(steps[0].new_total_shards, 1);
    // Shard 1 (now 0) gains keys that were on shard 0; with 1 shard, it owns all
    assert!(!steps[0].keys_to_import.is_empty());
}
