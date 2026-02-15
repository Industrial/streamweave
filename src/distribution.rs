//! Built-in distribution layer for cluster sharding.
//!
//! Runs N graph instances (shards) in one process, routes input by partition key,
//! and merges output. See [cluster-sharding.md](../docs/cluster-sharding.md) §7.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use streamweave::graph::Graph;
//! use streamweave::distribution::ShardedRunner;
//! use streamweave::partitioning::{PartitioningConfig, partition_by_key, PartitionKey};
//! use std::sync::Arc;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let total_shards = 4u32;
//! let partitioning: PartitioningConfig = partition_by_key(|payload: Arc<dyn std::any::Any + Send + Sync>| {
//!     Box::pin(async move {
//!         if let Ok(map) = payload.downcast::<std::collections::HashMap<String, String>>() {
//!             Ok(PartitionKey::from(map.get("key").cloned().unwrap_or_default()))
//!         } else {
//!             Err("Expected HashMap with key".to_string())
//!         }
//!     })
//! });
//!
//! let mut runner = ShardedRunner::new(
//!     total_shards,
//!     partitioning,
//!     |shard_id, total| {
//!         let mut g = Graph::new(format!("shard_{}", shard_id));
//!         // ... add nodes, edges, expose ports ...
//!         g.set_shard_config(shard_id, total);
//!         g
//!     },
//!     "input",
//!     "output",
//! )?;
//!
//! runner.execute().await?;
//! // Send to runner.input_tx(), receive from runner.output_rx()
//! runner.wait_for_completion().await?;
//! # Ok(())
//! # }
//! ```

use crate::graph::{Graph, GraphExecutionError};
use crate::partitioning::PartitioningConfig;
use crate::rebalance::ShardAssignment;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Runner that deploys N graph instances, routes input by partition key, and merges output.
///
/// Each shard runs the same graph topology with `set_shard_config(shard_id, total_shards)`.
/// Input is routed by `hash(key) % total_shards`; output from all shards is merged.
pub struct ShardedRunner {
  /// Unified input sender; caller sends here; router forwards to shards.
  input_tx: mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>,
  /// Merged output receiver; all shards send to a shared sink.
  output_rx: mpsc::Receiver<Arc<dyn std::any::Any + Send + Sync>>,
  /// Running graph instances (for wait_for_completion).
  graphs: Vec<Graph>,
}

impl ShardedRunner {
  /// Creates a sharded runner with N graph instances.
  ///
  /// # Arguments
  ///
  /// * `total_shards` - Number of shards (graph instances)
  /// * `partitioning` - Config for extracting partition key from each input
  /// * `factory` - Builds a graph for each shard; call `set_shard_config` is done here
  /// * `input_port` - External input port name on each graph
  /// * `output_port` - External output port name on each graph
  ///
  /// # Errors
  ///
  /// Returns an error if any graph lacks the exposed ports or if connection fails.
  pub fn new<F>(
    total_shards: u32,
    partitioning: PartitioningConfig,
    factory: F,
    input_port: &str,
    output_port: &str,
  ) -> Result<Self, String>
  where
    F: FnMut(u32, u32) -> Graph,
  {
    Self::new_with_capacity(
      total_shards,
      partitioning,
      factory,
      input_port,
      output_port,
      64,
    )
  }

  /// Like [`new`](Self::new) but with configurable channel capacity.
  pub fn new_with_capacity<F>(
    total_shards: u32,
    partitioning: PartitioningConfig,
    mut factory: F,
    input_port: &str,
    output_port: &str,
    channel_capacity: usize,
  ) -> Result<Self, String>
  where
    F: FnMut(u32, u32) -> Graph,
  {
    if total_shards == 0 {
      return Err("total_shards must be >= 1".to_string());
    }

    let (input_tx, input_rx) = mpsc::channel(channel_capacity);
    let (output_tx, output_rx) = mpsc::channel(channel_capacity);

    let mut shard_txs: Vec<mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>> = Vec::new();
    let mut graphs: Vec<Graph> = Vec::new();

    for shard_id in 0..total_shards {
      let (shard_tx, shard_rx) = mpsc::channel(channel_capacity);
      shard_txs.push(shard_tx);

      let mut graph = factory(shard_id, total_shards);
      graph.set_shard_config(shard_id, total_shards);
      graph
        .connect_input_channel(input_port, shard_rx)
        .map_err(|e| format!("Shard {} input connect: {}", shard_id, e))?;
      graph
        .connect_output_channel(output_port, output_tx.clone())
        .map_err(|e| format!("Shard {} output connect: {}", shard_id, e))?;
      graphs.push(graph);
    }

    // Spawn router: input_rx -> extract key -> hash % N -> shard_tx
    let assignment = ShardAssignment::new(0, total_shards); // shard_for_key only needs total_shards
    tokio::spawn(router_task(input_rx, shard_txs, partitioning, assignment));

    Ok(Self {
      input_tx,
      output_rx,
      graphs,
    })
  }

  /// Returns a clone of the input sender for sending data into the sharded graph.
  pub fn input_tx(&self) -> mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>> {
    self.input_tx.clone()
  }

  /// Returns a mutable reference to the output receiver.
  pub fn output_rx_mut(&mut self) -> &mut mpsc::Receiver<Arc<dyn std::any::Any + Send + Sync>> {
    &mut self.output_rx
  }

  /// Starts execution of all shard graphs.
  pub async fn execute(&mut self) -> Result<(), GraphExecutionError> {
    for graph in &mut self.graphs {
      graph.execute().await?;
    }
    Ok(())
  }

  /// Waits for all shard graphs to complete.
  pub async fn wait_for_completion(&mut self) -> Result<(), GraphExecutionError> {
    for graph in &mut self.graphs {
      graph.wait_for_completion().await?;
    }
    Ok(())
  }

  /// Stops all shard graphs.
  pub async fn stop(&self) -> Result<(), GraphExecutionError> {
    for graph in &self.graphs {
      graph.stop().await?;
    }
    Ok(())
  }

  /// Returns the number of shards.
  pub fn shard_count(&self) -> usize {
    self.graphs.len()
  }
}

async fn router_task(
  mut input_rx: mpsc::Receiver<Arc<dyn std::any::Any + Send + Sync>>,
  shard_txs: Vec<mpsc::Sender<Arc<dyn std::any::Any + Send + Sync>>>,
  partitioning: PartitioningConfig,
  assignment: ShardAssignment,
) {
  while let Some(payload) = input_rx.recv().await {
    let shard_id = match partitioning
      .key_extractor
      .extract_key(payload.clone())
      .await
    {
      Ok(key) => assignment.shard_for_key(key.as_str()),
      Err(_) => 0u32, // fallback to shard 0 on extract error
    };
    if (shard_id as usize) < shard_txs.len() {
      let _ = shard_txs[shard_id as usize].send(payload).await;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::partitioning::PartitionKey;
  use crate::partitioning::partition_by_key;
  use std::collections::HashMap;

  #[tokio::test]
  async fn test_sharded_runner_new_and_shard_count() {
    let partitioning: PartitioningConfig =
      partition_by_key(|p: Arc<dyn std::any::Any + Send + Sync>| {
        Box::pin(async move {
          if let Ok(m) = p.downcast::<HashMap<String, String>>() {
            Ok(PartitionKey::from(m.get("k").cloned().unwrap_or_default()))
          } else {
            Err("bad".to_string())
          }
        })
      });
    let runner = ShardedRunner::new(
      4,
      partitioning,
      |shard_id, total| {
        let mut g = Graph::new(format!("s{}", shard_id));
        g.add_node(
          "map".to_string(),
          Box::new(crate::nodes::map_node::MapNode::new("map".to_string())),
        )
        .unwrap();
        g.expose_input_port("map", "in", "input").unwrap();
        g.expose_output_port("map", "out", "output").unwrap();
        g.set_shard_config(shard_id, total);
        g
      },
      "input",
      "output",
    )
    .unwrap();
    assert_eq!(runner.shard_count(), 4);
  }

  #[test]
  fn test_sharded_runner_invalid_total_shards() {
    let partitioning: PartitioningConfig =
      partition_by_key(|_p: Arc<dyn std::any::Any + Send + Sync>| {
        Box::pin(async { Ok(PartitionKey::from("x")) })
      });
    let result: Result<ShardedRunner, _> = ShardedRunner::new(
      0,
      partitioning,
      |sid, tot| {
        let mut g = Graph::new(format!("s{}", sid));
        g.set_shard_config(sid, tot);
        g
      },
      "input",
      "output",
    );
    if let Err(e) = result {
      assert!(e.contains("total_shards"));
    } else {
      panic!("expected Err for total_shards=0");
    }
  }
}
