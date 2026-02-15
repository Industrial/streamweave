//! # Shard config (in-process demo)
//!
//! Demonstrates [`ShardConfig`], [`owns_key`](ShardConfig::owns_key), and
//! [`export_state_for_keys`](streamweave::graph::Graph::export_state_for_keys)
//! without a full cluster: one graph instance with a shard identity, key routing,
//! and state export API.
//!
//! See [docs/cluster-sharding.md](../docs/cluster-sharding.md).

use streamweave::graph::Graph;
use streamweave::nodes::variable_node::VariableNode;
use streamweave::partitioning::PartitionKey;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!("Shard config example (owns_key, export_state in-process)\n");

  let mut graph = Graph::new("shard_demo".to_string());
  graph
    .add_node(
      "n".to_string(),
      Box::new(VariableNode::new("n".to_string())),
    )
    .unwrap();

  // Run as "shard 1 of 3": only keys that hash to 1 are owned by this instance.
  graph.set_shard_config(1, 3);
  let config = graph.shard_config().expect("shard config set");
  println!(
    "ShardConfig: shard_id={}, total_shards={}",
    config.shard_id, config.total_shards
  );

  // Demonstrate owns_key: which keys belong to this shard?
  for key in ["alice", "bob", "carol", "dave", "eve"] {
    let owns = config.owns_key(key);
    println!("  owns_key(\"{}\") = {}", key, owns);
  }

  // Export state for keys (e.g. for rebalance); nodes without key-scoped state return empty.
  let keys = [
    PartitionKey::new("alice".to_string()),
    PartitionKey::new("bob".to_string()),
  ];
  let data = graph.export_state_for_keys("n", &keys)?;
  println!(
    "\nExport state for keys [alice, bob]: {} bytes (node has no key-scoped state)",
    data.len()
  );

  println!("\nDone.");
  Ok(())
}
