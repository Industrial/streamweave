//! # Shard config (in-process demo)
//!
//! Demonstrates [`ShardConfig`], [`owns_key`](ShardConfig::owns_key), and
//! [`export_state_for_keys`](streamweave::graph::Graph::export_state_for_keys)
//! without a full cluster. Graph is loaded from [shard_config.mmd](shard_config.mmd).
//!
//! See [docs/cluster-sharding.md](../docs/cluster-sharding.md).

mod run {
  use std::path::Path;
  use streamweave::mermaid::{
  blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint, NodeRegistry,
};
  use streamweave::nodes::variable_node::VariableNode;
  use streamweave::partitioning::PartitionKey;

  pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Shard config example (owns_key, export_state in-process)\n");

    let mmd_path = Path::new("examples/shard_config.mmd");
    let bp = parse_mmd_file_to_blueprint(mmd_path)?;
    let mut registry = NodeRegistry::new();
    registry.register("VariableNode", |id, _inputs, _outputs| {
      Box::new(VariableNode::new(id))
    });
    let graph = blueprint_to_graph(&bp, Some(&registry))?;
    let config = graph.shard_config().expect("shard config set");
    println!(
      "ShardConfig: shard_id={}, total_shards={}",
      config.shard_id, config.total_shards
    );

    for key in ["alice", "bob", "carol", "dave", "eve"] {
      let owns = config.owns_key(key);
      println!("  owns_key(\"{}\") = {}", key, owns);
    }

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
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  run::run()
}
