//! Export a graph blueprint to Mermaid (`.mmd`) string.
//!
//! When the `mermaid` feature is enabled, uses the `mermaid-builder` crate to build the flowchart.
//! When the feature is off, uses the hand-written format from [`GraphBlueprint::to_mermaid_string`]
//! so that string node ids are preserved and no extra dependency is required.
//!
//! Optionally, [`write_blueprint_to_path`] writes both the `.mmd` file and a co-located
//! `*.streamweave.yaml` sidecar so that roundtrip (parse → export → parse) preserves structure.

use crate::mermaid::blueprint::GraphBlueprint;
use std::path::Path;

#[cfg(feature = "mermaid")]
use crate::mermaid::convention;

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
    #[cfg(feature = "mermaid")]
    return blueprint_to_mermaid_with_builder(bp);

    #[cfg(not(feature = "mermaid"))]
    bp.to_mermaid_string()
}

/// Error when writing blueprint to path (mmd or sidecar).
#[derive(Debug, thiserror::Error)]
pub enum ExportPathError {
    /// I/O error writing the `.mmd` or sidecar file.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Sidecar write failed (e.g. when `mermaid` feature is off).
    #[cfg(feature = "mermaid")]
    #[error("sidecar: {0}")]
    Sidecar(#[from] crate::mermaid::sidecar::SidecarError),
}

/// Writes the blueprint to `path` as `.mmd` and to a co-located `*.streamweave.yaml` sidecar.
///
/// `path` is the intended `.mmd` path (e.g. `pipeline.mmd`). The sidecar is written to
/// `pipeline.streamweave.yaml`. When the `mermaid` feature is disabled, only the `.mmd` content
/// is written (using the hand-written format); sidecar write is skipped and no error is returned.
pub fn write_blueprint_to_path(
    path: &Path,
    bp: &GraphBlueprint,
) -> Result<(), ExportPathError> {
    let mmd = blueprint_to_mermaid(bp);
    std::fs::write(path, mmd)?;
    #[cfg(feature = "mermaid")]
    {
        let sidecar_path = path.with_extension("streamweave.yaml");
        crate::mermaid::sidecar::write_sidecar(&sidecar_path, bp)?;
    }
    Ok(())
}

#[cfg(feature = "mermaid")]
fn blueprint_to_mermaid_with_builder(bp: &GraphBlueprint) -> String {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use mermaid_builder::prelude::*;

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
    // Deterministic order for node_id mapping (mermaid-builder uses v0, v1, ...)
    let node_ids: Vec<String> = {
        let mut v: Vec<String> = bp.nodes.keys().cloned().collect();
        v.sort();
        v
    };
    for (i, nid) in node_ids.iter().enumerate() {
        out.push_str(&format!("%% streamweave: node_id v{}={}\n", i, nid));
    }
    out.push_str("%% streamweave: end\n");

    let mut builder = FlowchartBuilder::default();
    let mut node_rcs: BTreeMap<String, Rc<FlowchartNode>> = BTreeMap::new();
    for (i, node_id) in node_ids.iter().enumerate() {
        let info = bp.nodes.get(node_id).map_or_else(
            crate::mermaid::blueprint::NodeInfo::default,
            std::clone::Clone::clone,
        );
        let label = info
            .label
            .as_deref()
            .unwrap_or(node_id)
            .replace('"', "\\\"");
        let node_builder = FlowchartNodeBuilder::default()
            .id(i as u64)
            .label(&label)
            .expect("node label");
        let node = builder.node(node_builder).expect("add node");
        node_rcs.insert(node_id.clone(), node);
    }
    for e in &bp.edges {
        let src = node_rcs
            .get(&e.source_node)
            .expect("source node in map")
            .clone();
        let dst = node_rcs
            .get(&e.target_node)
            .expect("target node in map")
            .clone();
        let label = convention::format_edge_label(&e.source_port, &e.target_port);
        let edge_builder = FlowchartEdgeBuilder::default()
            .source(src)
            .expect("edge source")
            .destination(dst)
            .expect("edge dest")
            .label(label)
            .expect("edge label");
        builder.edge(edge_builder).expect("add edge");
    }
    let flowchart: Flowchart = builder.into();
    out.push_str(&flowchart.to_string());
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
        assert!(
            s.contains("out->in") || s.contains("a -->|out->in| b") || s.contains("-->|out->in|"),
            "edge label must use convention"
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
        assert!(s.contains("out->in1"));
        assert!(s.contains("out->in2"));
        assert!(s.contains("out->in3"));
    }

    #[test]
    fn export_from_graph_produces_valid_mmd() {
        use crate::edge::Edge;
        use crate::graph::Graph;
        use crate::nodes::variable_node::VariableNode;

        let mut graph = Graph::new("g".to_string());
        graph
            .add_node("src".to_string(), Box::new(VariableNode::new("src".to_string())))
            .expect("add src");
        graph
            .add_node("dst".to_string(), Box::new(VariableNode::new("dst".to_string())))
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
        assert!(s.contains("out->value"));
    }
}
