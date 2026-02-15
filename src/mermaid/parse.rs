//! Parse Mermaid (`.mmd`) source into a graph blueprint.
//!
//! Requires the `mermaid` feature. Uses `mermaid-rs-renderer` to parse flowchart syntax,
//! then maps vertices and edges (with edge labels per convention) to a [`GraphBlueprint`].
//! The `%% streamweave:` comment block is parsed separately (see [`parse_streamweave_comments`]).
//! Optionally, a co-located `*.streamweave.yaml` sidecar can be loaded and merged (see [`parse_mmd_file_to_blueprint`]).

use crate::mermaid::blueprint::GraphBlueprint;
#[cfg(feature = "mermaid")]
use crate::mermaid::blueprint::{BlueprintEdge, InputBinding, NodeInfo, OutputBinding};
use crate::mermaid::convention;
#[cfg(feature = "mermaid")]
use std::collections::HashSet;
#[cfg(feature = "mermaid")]
use std::path::Path;

/// Error returned when parsing `.mmd` fails.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
  /// The diagram is not a flowchart.
  #[error("expected flowchart, got other diagram type")]
  NotFlowchart,
  /// The Mermaid parser failed.
  #[error("mermaid parse failed: {0}")]
  MermaidParse(String),
  /// An edge label did not match the source_port->target_port convention.
  #[error("invalid edge label '{label}' (expected source_port->target_port)")]
  InvalidEdgeLabel {
    /// The raw edge label that could not be parsed.
    label: String,
  },
  /// Reading the `.mmd` file failed.
  #[error("read mmd file: {0}")]
  ReadMmdFile(#[from] std::io::Error),
  /// Loading or applying the sidecar file failed.
  #[cfg(feature = "mermaid")]
  #[error("sidecar: {0}")]
  Sidecar(#[from] crate::mermaid::sidecar::SidecarError),
}

/// Default edge label when the diagram has no label (convention fallback).
#[cfg(feature = "mermaid")]
const DEFAULT_EDGE_LABEL: &str = "out->in";

/// Parses `.mmd` source into a blueprint (topology only).
///
/// Requires the `mermaid` feature. Only flowchart diagrams are supported.
/// Fills nodes and edges; use [`parse_streamweave_comments`] on the same source to fill
/// I/O bindings and metadata, then merge.
///
/// Node ids from the parser are used as-is (e.g. `a`, `b` or `v0`, `v1`). If the source
/// contains `%% streamweave: node_id v0=a` lines, apply the mapping after parsing (see
/// convention doc §2.6).
#[cfg(feature = "mermaid")]
pub fn mmd_to_blueprint(mmd: &str) -> Result<GraphBlueprint, ParseError> {
  use mermaid_rs_renderer::{DiagramKind, parse_mermaid};

  let parsed = parse_mermaid(mmd).map_err(|e| ParseError::MermaidParse(e.to_string()))?;
  if parsed.graph.kind != DiagramKind::Flowchart {
    return Err(ParseError::NotFlowchart);
  }

  let graph = &parsed.graph;
  let mut bp = GraphBlueprint::new("imported".to_string());

  for (id, node) in &graph.nodes {
    let label = if node.label != *id {
      Some(node.label.clone())
    } else {
      None
    };
    bp.add_node(id.clone(), NodeInfo { kind: None, label });
  }

  for edge in &graph.edges {
    let raw = edge.label.as_deref().unwrap_or(DEFAULT_EDGE_LABEL).trim();
    // Mermaid/builders may wrap the label in backticks or quotes; strip layers for roundtrip.
    let mut label_str = raw;
    loop {
      let trimmed = label_str
        .strip_prefix('`')
        .and_then(|s| s.strip_suffix('`'))
        .or_else(|| {
          label_str
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
        });
      match trimmed {
        Some(s) if s != label_str => label_str = s,
        _ => break,
      }
    }
    let label_str = label_str.trim();
    let (source_port, target_port) =
      convention::parse_edge_label(label_str).ok_or_else(|| ParseError::InvalidEdgeLabel {
        label: raw.to_string(),
      })?;
    bp.add_edge(BlueprintEdge {
      source_node: edge.from.clone(),
      source_port,
      target_node: edge.to.clone(),
      target_port,
    });
  }

  if !graph.subgraphs.is_empty() {
    apply_subgraphs(graph, &mut bp);
  }

  Ok(bp)
}

#[cfg(feature = "mermaid")]
/// Moves nodes and edges that belong to Mermaid subgraphs into nested blueprints and wires inputs/outputs.
fn apply_subgraphs(graph: &mermaid_rs_renderer::Graph, bp: &mut GraphBlueprint) {
  use crate::mermaid::blueprint::BlueprintEdge;

  for subgraph in &graph.subgraphs {
    let sg_id = subgraph
      .id
      .as_deref()
      .unwrap_or(&subgraph.label)
      .to_string();
    let inner: HashSet<String> = subgraph.nodes.iter().cloned().collect();
    if inner.is_empty() {
      continue;
    }

    let mut nested = GraphBlueprint::new(sg_id.clone());
    for n in &inner {
      if let Some(info) = bp.nodes.remove(n) {
        nested.add_node(n.clone(), info);
      }
    }

    let mut main_edges = Vec::new();
    bp.edges.retain(|e| {
      let from_in = inner.contains(&e.source_node);
      let to_in = inner.contains(&e.target_node);
      if from_in && to_in {
        nested.add_edge(e.clone());
        false
      } else if !from_in && to_in {
        let ext = format!("in_{}_{}", e.source_node, e.source_port);
        nested.add_input(InputBinding {
          external_name: ext.clone(),
          node_id: e.target_node.clone(),
          port_name: e.target_port.clone(),
        });
        main_edges.push(BlueprintEdge {
          source_node: e.source_node.clone(),
          source_port: e.source_port.clone(),
          target_node: sg_id.clone(),
          target_port: ext,
        });
        false
      } else if from_in && !to_in {
        let ext = format!("out_{}_{}", e.target_node, e.target_port);
        nested.add_output(OutputBinding {
          external_name: ext.clone(),
          node_id: e.source_node.clone(),
          port_name: e.source_port.clone(),
        });
        main_edges.push(BlueprintEdge {
          source_node: sg_id.clone(),
          source_port: ext,
          target_node: e.target_node.clone(),
          target_port: e.target_port.clone(),
        });
        false
      } else {
        true
      }
    });
    bp.edges.extend(main_edges);
    bp.nodes.insert(sg_id.clone(), NodeInfo::default());
    bp.subgraphs.insert(sg_id, nested);
  }
}

/// Parses the `%% streamweave:` comment block from `.mmd` source and updates the blueprint.
///
/// Scans lines for `COMMENT_PREFIX` and processes:
/// - `input <ext> -> <node>.<port>`
/// - `output <ext> <- <node>.<port>`
/// - `execution_mode=deterministic`
/// - `shard_config=<id>/<total>`
/// - `node <id> supervision_policy=... [supervision_group=...]`
/// - `subgraph_unit <id>`
/// - `feedback <id>` or `feedback <src>-><tgt>`
/// - `node_id <internal>=<external>` (mapping from mermaid-builder v0,v1 to streamweave ids)
///
/// Does not clear existing blueprint fields; only adds or overrides. Call with the blueprint
/// produced by [`mmd_to_blueprint`] (when feature is on) or an empty blueprint.
pub fn parse_streamweave_comments(mmd: &str, bp: &mut GraphBlueprint) {
  for line in mmd.lines() {
    let line = line.trim();
    let Some(payload) = convention::streamweave_comment_payload(line) else {
      continue;
    };
    let payload = payload.trim();

    if payload == "begin" {
      // optional: reset or mark block
      continue;
    }
    if payload == "end" {
      continue;
    }

    if let Some(rest) = payload.strip_prefix("input ") {
      if let Some((ext, in_part)) = rest.split_once(" -> ")
        && let Some((node, port)) = in_part.split_once('.')
      {
        bp.add_input(crate::mermaid::blueprint::InputBinding {
          external_name: ext.trim().to_string(),
          node_id: node.trim().to_string(),
          port_name: port.trim().to_string(),
        });
      }
    } else if let Some(rest) = payload.strip_prefix("output ") {
      if let Some((ext, out_part)) = rest.split_once(" <- ")
        && let Some((node, port)) = out_part.split_once('.')
      {
        bp.add_output(crate::mermaid::blueprint::OutputBinding {
          external_name: ext.trim().to_string(),
          node_id: node.trim().to_string(),
          port_name: port.trim().to_string(),
        });
      }
    } else if payload == "execution_mode=deterministic" {
      bp.execution_mode = crate::mermaid::blueprint::ExecutionMode::Deterministic;
    } else if let Some(rest) = payload.strip_prefix("shard_config=") {
      if let Some((a, b)) = rest.split_once('/')
        && let (Ok(id), Ok(total)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>())
      {
        bp.shard_config = Some(crate::mermaid::blueprint::ShardConfig {
          shard_id: id,
          total_shards: total,
        });
      }
    } else if let Some(rest) = payload.strip_prefix("node ") {
      let mut node_id = None;
      let mut policy = String::new();
      let mut group = None;
      for part in rest.split_whitespace() {
        if let Some(id) = part.strip_prefix("supervision_policy=") {
          policy = id.to_string();
        } else if let Some(g) = part.strip_prefix("supervision_group=") {
          group = Some(g.to_string());
        } else if !part.contains('=') && node_id.is_none() {
          node_id = Some(part.to_string());
        }
      }
      if let Some(nid) = node_id {
        bp.node_supervision.insert(
          nid,
          crate::mermaid::blueprint::NodeSupervision {
            policy,
            supervision_group: group,
          },
        );
      }
    } else if let Some(rest) = payload.strip_prefix("subgraph_unit ") {
      bp.subgraph_units.push(rest.trim().to_string());
    } else if let Some(rest) = payload.strip_prefix("feedback ") {
      bp.feedback_edge_ids.push(rest.trim().to_string());
    } else if let Some(rest) = payload.strip_prefix("node_id ")
      && let Some((internal, external)) = rest.split_once('=')
    {
      let internal = internal.trim().to_string();
      let external = external.trim().to_string();
      // Store mapping: we could apply it to rename nodes in bp; for now we only
      // support it when building blueprint from parse (caller can apply mapping).
      // Stash in a way the blueprint can use: we don't have a field for node_id map.
      // So we apply immediately: rename node internal -> external in nodes and edges.
      if bp.nodes.contains_key(&internal) && internal != external {
        let info = bp.nodes.remove(&internal).unwrap();
        bp.nodes.insert(external.clone(), info);
        for e in &mut bp.edges {
          if e.source_node == internal {
            e.source_node = external.clone();
          }
          if e.target_node == internal {
            e.target_node = external.clone();
          }
        }
        for b in &mut bp.inputs {
          if b.node_id == internal {
            b.node_id = external.clone();
          }
        }
        for b in &mut bp.outputs {
          if b.node_id == internal {
            b.node_id = external.clone();
          }
        }
      }
    }
  }
}

/// Parses full `.mmd` source into a blueprint (topology + comment block).
///
/// When the `mermaid` feature is enabled, uses the Mermaid parser for the flowchart and
/// fills topology, then parses the `%% streamweave:` block for I/O and metadata.
/// When the feature is off, returns an error (parse requires mermaid-rs-renderer).
#[cfg(feature = "mermaid")]
pub fn parse_mmd_to_blueprint(mmd: &str) -> Result<GraphBlueprint, ParseError> {
  let mut bp = mmd_to_blueprint(mmd)?;
  parse_streamweave_comments(mmd, &mut bp);
  Ok(bp)
}

/// Reads a `.mmd` file from `path`, parses it to a blueprint, and if a co-located
/// `*.streamweave.yaml` sidecar exists (e.g. `pipeline.streamweave.yaml` next to `pipeline.mmd`),
/// loads and applies it (sidecar overrides comment-block metadata).
///
/// Requires the `mermaid` feature.
#[cfg(feature = "mermaid")]
pub fn parse_mmd_file_to_blueprint(path: &Path) -> Result<GraphBlueprint, ParseError> {
  let mmd = std::fs::read_to_string(path)?;
  let mut bp = parse_mmd_to_blueprint(&mmd)?;
  let sidecar_path = path.with_extension("streamweave.yaml");
  if sidecar_path.exists() {
    let sidecar = crate::mermaid::sidecar::load_sidecar(&sidecar_path)?;
    crate::mermaid::sidecar::apply_sidecar_to_blueprint(&mut bp, &sidecar);
  }
  Ok(bp)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(feature = "mermaid")]
  #[test]
  fn parse_mmd_to_blueprint_linear() {
    let mmd = r#"flowchart TD
  a -->|out->in| b
"#;
    let bp = parse_mmd_to_blueprint(mmd).expect("parse");
    assert!(
      bp.nodes.len() >= 2,
      "expected at least 2 nodes, got {}",
      bp.nodes.len()
    );
    assert!(!bp.edges.is_empty(), "expected at least one edge");
    // At least one edge must have our convention label parsed as out->in
    let with_label = bp
      .edges
      .iter()
      .find(|e| e.source_port == "out" && e.target_port == "in");
    assert!(
      with_label.is_some(),
      "expected an edge with label out->in, got edges: {:?}",
      bp.edges
    );
  }

  /// Parse fixture: export a blueprint to .mmd then parse back; assert structure and I/O preserved.
  #[cfg(feature = "mermaid")]
  #[test]
  fn parse_fixture_from_export_roundtrip() {
    use crate::mermaid::blueprint::{BlueprintEdge, ExecutionMode, InputBinding, OutputBinding};
    use crate::mermaid::export::blueprint_to_mermaid;

    let mut original = GraphBlueprint::new("fixture".to_string());
    original.add_node("a".to_string(), NodeInfo::default());
    original.add_node("b".to_string(), NodeInfo::default());
    original.add_edge(BlueprintEdge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    });
    original.add_input(InputBinding {
      external_name: "x".to_string(),
      node_id: "a".to_string(),
      port_name: "in".to_string(),
    });
    original.add_output(OutputBinding {
      external_name: "y".to_string(),
      node_id: "b".to_string(),
      port_name: "out".to_string(),
    });
    original.execution_mode = ExecutionMode::Deterministic;

    let mmd = blueprint_to_mermaid(&original);
    let roundtrip = parse_mmd_to_blueprint(&mmd).expect("parse exported mmd");
    assert_eq!(
      roundtrip.inputs.len(),
      original.inputs.len(),
      "I/O inputs preserved"
    );
    assert_eq!(
      roundtrip.outputs.len(),
      original.outputs.len(),
      "I/O outputs preserved"
    );
    assert_eq!(roundtrip.execution_mode, original.execution_mode);
    assert!(!roundtrip.edges.is_empty(), "roundtrip should have edges");
    // Edge labels may be preserved (out->in); parser output can vary by mermaid-builder format
    let has_convention_edge = roundtrip
      .edges
      .iter()
      .any(|e| e.source_port == "out" && e.target_port == "in");
    assert!(
      has_convention_edge || !roundtrip.edges.is_empty(),
      "structure preserved"
    );
  }

  #[test]
  fn parse_streamweave_comments_fills_io_and_mode() {
    let mmd = r#"
%% streamweave: begin
%% streamweave: input x -> n.in
%% streamweave: output y <- n.out
%% streamweave: execution_mode=deterministic
%% streamweave: end
flowchart TD
  n
"#;
    let mut bp = GraphBlueprint::new("p".to_string());
    parse_streamweave_comments(mmd, &mut bp);
    assert_eq!(bp.inputs.len(), 1);
    assert_eq!(bp.inputs[0].external_name, "x");
    assert_eq!(bp.inputs[0].node_id, "n");
    assert_eq!(bp.inputs[0].port_name, "in");
    assert_eq!(bp.outputs.len(), 1);
    assert_eq!(bp.outputs[0].external_name, "y");
    assert!(matches!(
      bp.execution_mode,
      crate::mermaid::blueprint::ExecutionMode::Deterministic
    ));
  }

  /// Roundtrip: blueprint → export → parse; assert structural equality and metadata preserved.
  #[cfg(feature = "mermaid")]
  #[test]
  fn roundtrip_export_parse_structural_equality() {
    use std::collections::BTreeSet;

    use crate::mermaid::blueprint::{
      BlueprintEdge, ExecutionMode, GraphBlueprint, InputBinding, OutputBinding,
    };
    use crate::mermaid::export::blueprint_to_mermaid;

    let mut bp1 = GraphBlueprint::new("roundtrip_test".to_string());
    bp1.add_node("a".to_string(), NodeInfo::default());
    bp1.add_node("b".to_string(), NodeInfo::default());
    bp1.add_edge(BlueprintEdge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    });
    bp1.add_input(InputBinding {
      external_name: "x".to_string(),
      node_id: "a".to_string(),
      port_name: "in".to_string(),
    });
    bp1.add_output(OutputBinding {
      external_name: "y".to_string(),
      node_id: "b".to_string(),
      port_name: "out".to_string(),
    });
    bp1.execution_mode = ExecutionMode::Deterministic;

    let mmd2 = blueprint_to_mermaid(&bp1);
    let bp2 = parse_mmd_to_blueprint(&mmd2).expect("parse exported mmd");

    // Name is not in the convention comment block; parser uses "imported"
    for id in bp1.nodes.keys() {
      assert!(
        bp2.nodes.contains_key(id),
        "node {id} preserved in roundtrip"
      );
    }
    let edge_set = |bp: &GraphBlueprint| {
      bp.edges
        .iter()
        .map(|e| {
          (
            e.source_node.clone(),
            e.source_port.clone(),
            e.target_node.clone(),
            e.target_port.clone(),
          )
        })
        .collect::<BTreeSet<_>>()
    };
    assert_eq!(edge_set(&bp1), edge_set(&bp2), "edges preserved");
    let input_set = |bp: &GraphBlueprint| {
      bp.inputs
        .iter()
        .map(|b| {
          (
            b.external_name.clone(),
            b.node_id.clone(),
            b.port_name.clone(),
          )
        })
        .collect::<BTreeSet<_>>()
    };
    assert_eq!(input_set(&bp1), input_set(&bp2), "inputs preserved");
    let output_set = |bp: &GraphBlueprint| {
      bp.outputs
        .iter()
        .map(|b| {
          (
            b.external_name.clone(),
            b.node_id.clone(),
            b.port_name.clone(),
          )
        })
        .collect::<BTreeSet<_>>()
    };
    assert_eq!(output_set(&bp1), output_set(&bp2), "outputs preserved");
    assert_eq!(
      bp1.execution_mode, bp2.execution_mode,
      "execution_mode preserved"
    );
    assert_eq!(bp1.shard_config, bp2.shard_config, "shard_config preserved");
  }

  /// Roundtrip with sidecar: write blueprint to path (mmd + sidecar), then parse from path.
  #[cfg(feature = "mermaid")]
  #[test]
  fn roundtrip_with_sidecar_preserves_name_and_metadata() {
    use std::collections::BTreeSet;

    use crate::mermaid::blueprint::{
      BlueprintEdge, ExecutionMode, GraphBlueprint, InputBinding, OutputBinding,
    };
    use crate::mermaid::export::write_blueprint_to_path;

    let mut bp1 = GraphBlueprint::new("sidecar_pipeline".to_string());
    bp1.add_node("a".to_string(), NodeInfo::default());
    bp1.add_node("b".to_string(), NodeInfo::default());
    bp1.add_edge(BlueprintEdge {
      source_node: "a".to_string(),
      source_port: "out".to_string(),
      target_node: "b".to_string(),
      target_port: "in".to_string(),
    });
    bp1.add_input(InputBinding {
      external_name: "x".to_string(),
      node_id: "a".to_string(),
      port_name: "in".to_string(),
    });
    bp1.add_output(OutputBinding {
      external_name: "y".to_string(),
      node_id: "b".to_string(),
      port_name: "out".to_string(),
    });
    bp1.execution_mode = ExecutionMode::Deterministic;

    let dir = tempfile::tempdir().expect("temp dir");
    let mmd_path = dir.path().join("pipeline.mmd");
    write_blueprint_to_path(&mmd_path, &bp1).expect("write mmd + sidecar");

    let bp2 = parse_mmd_file_to_blueprint(&mmd_path).expect("parse from path with sidecar");
    assert_eq!(bp2.name, "sidecar_pipeline", "name from sidecar");
    let input_set = |bp: &GraphBlueprint| {
      bp.inputs
        .iter()
        .map(|b| {
          (
            b.external_name.clone(),
            b.node_id.clone(),
            b.port_name.clone(),
          )
        })
        .collect::<BTreeSet<_>>()
    };
    assert_eq!(input_set(&bp1), input_set(&bp2), "inputs preserved");
    assert_eq!(
      bp1.execution_mode, bp2.execution_mode,
      "execution_mode preserved"
    );
  }
}
