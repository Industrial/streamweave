//! Export a graph blueprint to Mermaid (`.mmd`) string.
//!
//! Uses the `mermaid-builder` crate to build the flowchart. Optionally, [`write_blueprint_to_path`]
//! writes both the `.mmd` file and a co-located `*.streamweave.yaml` sidecar so that roundtrip
//! (parse → export → parse) preserves structure.

use crate::mermaid::blueprint::GraphBlueprint;
use crate::mermaid::convention;
use std::path::Path;

/// Converts a live graph to a Mermaid flowchart string per the StreamWeave convention.
///
/// Builds a blueprint from the graph’s topology, I/O bindings, and metadata, then exports it.
/// Node kinds/labels are not inferred; the blueprint uses default node info.
#[must_use]
pub fn graph_to_mermaid(graph: &crate::graph::Graph) -> String {
  blueprint_to_mermaid(&graph.to_blueprint())
}

/// Converts a blueprint to a Mermaid flowchart string per the StreamWeave convention.
///
/// Emits the `%% streamweave:` comment block (I/O, metadata, and when using mermaid-builder also
/// `node_id` mapping lines), then a `flowchart TD` diagram. Edge labels use `source_port->target_port`.
#[must_use]
pub fn blueprint_to_mermaid(bp: &GraphBlueprint) -> String {
  blueprint_to_mermaid_with_builder(bp)
}

/// Error when writing blueprint to path (mmd or sidecar).
#[derive(Debug, thiserror::Error)]
pub enum ExportPathError {
  /// I/O error writing the `.mmd` or sidecar file.
  #[error("io: {0}")]
  Io(#[from] std::io::Error),
  /// Sidecar write failed.
  #[error("sidecar: {0}")]
  Sidecar(#[from] crate::mermaid::sidecar::SidecarError),
}

/// Writes the blueprint to `path` as `.mmd` and to a co-located `*.streamweave.yaml` sidecar.
///
/// `path` is the intended `.mmd` path (e.g. `pipeline.mmd`). The sidecar is written to
/// `pipeline.streamweave.yaml`.
pub fn write_blueprint_to_path(path: &Path, bp: &GraphBlueprint) -> Result<(), ExportPathError> {
  let mmd = blueprint_to_mermaid(bp);
  std::fs::write(path, mmd)?;
  let sidecar_path = path.with_extension("streamweave.yaml");
  crate::mermaid::sidecar::write_sidecar(&sidecar_path, bp)?;
  Ok(())
}

/// Renders a blueprint to Mermaid diagram text (Style B: node-as-subgraph with port boxes).
fn blueprint_to_mermaid_with_builder(bp: &GraphBlueprint) -> String {
  let mut out = String::new();
  // Emit streamweave comment block (same content as blueprint, plus node_id mapping)
  out.push_str("%% streamweave: begin\n");
  for b in &bp.inputs {
    out.push_str(&format!(
      "%% streamweave: input {} -> {}.{}\n",
      b.external_name, b.node_id, b.port_name
    ));
  }
  for b in &bp.outputs {
    out.push_str(&format!(
      "%% streamweave: output {} <- {}.{}\n",
      b.external_name, b.node_id, b.port_name
    ));
  }
  match bp.execution_mode {
    crate::mermaid::blueprint::ExecutionMode::Concurrent => {}
    crate::mermaid::blueprint::ExecutionMode::Deterministic => {
      out.push_str("%% streamweave: execution_mode=deterministic\n");
    }
  }
  if let Some(s) = &bp.shard_config {
    out.push_str(&format!(
      "%% streamweave: shard_config={}/{}\n",
      s.shard_id, s.total_shards
    ));
  }
  for (node_id, sup) in &bp.node_supervision {
    let mut line = format!(
      "%% streamweave: node {} supervision_policy={}",
      node_id, sup.policy
    );
    if let Some(g) = &sup.supervision_group {
      line.push_str(&format!(" supervision_group={}", g));
    }
    line.push('\n');
    out.push_str(&line);
  }
  for sub_id in &bp.subgraph_units {
    out.push_str(&format!("%% streamweave: subgraph_unit {}\n", sub_id));
  }
  for fb in &bp.feedback_edge_ids {
    out.push_str(&format!("%% streamweave: feedback {}\n", fb));
  }
  let node_ids: Vec<String> = {
    let mut v: Vec<String> = bp.nodes.keys().cloned().collect();
    v.sort();
    v
  };
  for (_, nid) in node_ids.iter().enumerate() {
    if let Some(info) = bp.nodes.get(nid) {
      if let Some(ref k) = info.kind {
        out.push_str(&format!("%% streamweave: node {} kind={}\n", nid, k));
      }
    }
  }
  out.push_str("%% streamweave: end\n");

  // Style B: each node is a subgraph; port boxes inside; external I/O as nodes outside.
  out.push_str("flowchart TD\n");
  use std::collections::BTreeSet;

  // External I/O nodes (outside any subgraph)
  for b in &bp.inputs {
    let safe = convention::external_name_to_safe(&b.external_name);
    out.push_str(&format!("  {}[\"{}\"]\n", safe, b.external_name));
  }
  for b in &bp.outputs {
    let safe = convention::external_name_to_safe(&b.external_name);
    out.push_str(&format!("  {}[\"{}\"]\n", safe, b.external_name));
  }
  out.push('\n');

  // Per-node subgraph with port boxes and core
  for node_id in &node_ids {
    let info = bp.nodes.get(node_id).map_or_else(
      crate::mermaid::blueprint::NodeInfo::default,
      std::clone::Clone::clone,
    );
    let subgraph_label = info
      .kind
      .as_deref()
      .unwrap_or(node_id)
      .replace('"', "\\\"");
    let mut input_ports: BTreeSet<String> = BTreeSet::new();
    let mut output_ports: BTreeSet<String> = BTreeSet::new();
    for b in &bp.inputs {
      if b.node_id == *node_id {
        input_ports.insert(b.port_name.clone());
      }
    }
    for b in &bp.outputs {
      if b.node_id == *node_id {
        output_ports.insert(b.port_name.clone());
      }
    }
    for e in &bp.edges {
      if e.target_node == *node_id {
        input_ports.insert(e.target_port.clone());
      }
      if e.source_node == *node_id {
        output_ports.insert(e.source_port.clone());
      }
    }

    // Core node uses {node_id}_core to avoid Mermaid cycle (subgraph id same as node id).
    let core_id = format!("{}_core", node_id);
    out.push_str(&format!("  subgraph {}[\"{}\"]\n", node_id, subgraph_label));
    out.push_str("    direction LR\n");
    for p in &input_ports {
      let safe = convention::port_name_to_safe(p);
      let in_id = format!("{}_in_{}", node_id, safe);
      out.push_str(&format!("    {}[\"{}\"]\n", in_id, p));
    }
    out.push_str(&format!("    {}[\"{}\"]\n", core_id, node_id));
    for p in &output_ports {
      let safe = convention::port_name_to_safe(p);
      let out_id = format!("{}_out_{}", node_id, safe);
      out.push_str(&format!("    {}[\"{}\"]\n", out_id, p));
    }
    for p in &input_ports {
      let safe = convention::port_name_to_safe(p);
      let in_id = format!("{}_in_{}", node_id, safe);
      out.push_str(&format!("    {} --> {}\n", in_id, core_id));
    }
    for p in &output_ports {
      let safe = convention::port_name_to_safe(p);
      let out_id = format!("{}_out_{}", node_id, safe);
      out.push_str(&format!("    {} --> {}\n", core_id, out_id));
    }
    out.push_str("  end\n\n");
  }

  // Edges: external input -> port box; port box -> external output; port box -> port box (internal edges)
  for b in &bp.inputs {
    let ext_safe = convention::external_name_to_safe(&b.external_name);
    let in_id = format!("{}_in_{}", b.node_id, convention::port_name_to_safe(&b.port_name));
    out.push_str(&format!("  {} --> {}\n", ext_safe, in_id));
  }
  for b in &bp.outputs {
    let ext_safe = convention::external_name_to_safe(&b.external_name);
    let out_id = format!("{}_out_{}", b.node_id, convention::port_name_to_safe(&b.port_name));
    out.push_str(&format!("  {} --> {}\n", out_id, ext_safe));
  }
  for e in &bp.edges {
    let src_out = format!(
      "{}_out_{}",
      e.source_node,
      convention::port_name_to_safe(&e.source_port)
    );
    let tgt_in = format!(
      "{}_in_{}",
      e.target_node,
      convention::port_name_to_safe(&e.target_port)
    );
    out.push_str(&format!("  {} --> {}\n", src_out, tgt_in));
  }

  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::mermaid::blueprint::{
    BlueprintEdge, ExecutionMode, GraphBlueprint, InputBinding, NodeInfo, OutputBinding,
  };

  #[test]
  fn export_linear_two_nodes() {
    let mut bp = GraphBlueprint::new("test".to_string());
    bp.add_node("a".to_string(), NodeInfo::default());
    bp.add_node("b".to_string(), NodeInfo::default());
    bp.add_edge(BlueprintEdge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    });
    let s = blueprint_to_mermaid(&bp);
    assert!(s.contains("%% streamweave:"), "must contain comment block");
    assert!(
      s.contains("flowchart") || s.contains("graph"),
      "must contain flowchart or graph"
    );
    // Style B: subgraph per node, port boxes; edge from a_out_out to b_in_in
    assert!(s.contains("subgraph a"), "Style B: subgraph for node a");
    assert!(s.contains("subgraph b"), "Style B: subgraph for node b");
    assert!(
      s.contains("a_out_out") && s.contains("b_in_in"),
      "Style B: port box ids for edge out->in"
    );
  }

  #[test]
  fn export_includes_io_and_execution_mode() {
    let mut bp = GraphBlueprint::new("p".to_string());
    bp.add_node("n".to_string(), NodeInfo::default());
    bp.add_input(InputBinding {
      external_name: "x".to_string(),
      node_id: "n".to_string(),
      port_name: "in".to_string(),
    });
    bp.add_output(OutputBinding {
      external_name: "y".to_string(),
      node_id: "n".to_string(),
      port_name: "out".to_string(),
    });
    bp.execution_mode = ExecutionMode::Deterministic;
    let s = blueprint_to_mermaid(&bp);
    assert!(s.contains("input x -> n.in"));
    assert!(s.contains("output y <- n.out"));
    assert!(s.contains("execution_mode=deterministic"));
  }

  #[test]
  fn export_fan_in_three_sources_one_sink() {
    let mut bp = GraphBlueprint::new("fan".to_string());
    bp.add_node("a".to_string(), NodeInfo::default());
    bp.add_node("b".to_string(), NodeInfo::default());
    bp.add_node("c".to_string(), NodeInfo::default());
    bp.add_node("sink".to_string(), NodeInfo::default());
    bp.add_edge(BlueprintEdge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in1".to_string(),
    });
    bp.add_edge(BlueprintEdge {
      source_node: "b".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in2".to_string(),
    });
    bp.add_edge(BlueprintEdge {
      source_node: "c".to_string(),
      source_port: "out".to_string(),
      target_node: "sink".to_string(),
      target_port: "in3".to_string(),
    });
    let s = blueprint_to_mermaid(&bp);
    assert!(s.contains("%% streamweave:"));
    assert!(s.contains("flowchart") || s.contains("graph"));
    // Style B: edges via port boxes (sink_in_in1, etc.)
    assert!(s.contains("sink_in_in1"));
    assert!(s.contains("sink_in_in2"));
    assert!(s.contains("sink_in_in3"));
  }

  #[test]
  fn export_from_graph_produces_valid_mmd() {
    use crate::edge::Edge;
    use crate::graph::Graph;
    use crate::nodes::variable_node::VariableNode;

    let mut graph = Graph::new("g".to_string());
    graph
      .add_node(
        "src".to_string(),
        Box::new(VariableNode::new("src".to_string())),
      )
      .expect("add src");
    graph
      .add_node(
        "dst".to_string(),
        Box::new(VariableNode::new("dst".to_string())),
      )
      .expect("add dst");
    graph
      .add_edge(Edge {
        source_node: "src".to_string(),
        source_port: "out".to_string(),
        target_node: "dst".to_string(),
        target_port: "value".to_string(),
      })
      .expect("add edge");
    graph.expose_input_port("src", "value", "input").unwrap();
    graph.expose_output_port("dst", "out", "output").unwrap();

    let s = graph_to_mermaid(&graph);
    assert!(s.contains("%% streamweave: begin"));
    assert!(s.contains("%% streamweave: end"));
    assert!(s.contains("input input -> src.value"));
    assert!(s.contains("output output <- dst.out"));
    assert!(s.contains("flowchart") || s.contains("graph"));
    // Style B: edge from src_out_out to dst_in_value
    assert!(s.contains("src_out_out"));
    assert!(s.contains("dst_in_value"));
  }
}
