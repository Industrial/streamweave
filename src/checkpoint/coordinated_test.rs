//! Tests for coordinated checkpoint protocol.

use crate::checkpoint::{
    CheckpointCoordinator, CheckpointDone, CheckpointRequest, DistributedCheckpointStorage,
    FileDistributedCheckpointStorage, InMemoryCheckpointCoordinator,
};
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

    let req = CheckpointRequest {
        checkpoint_id: CheckpointId::new(42),
        barrier_t: None,
    };

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
