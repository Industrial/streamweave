//! Graph blueprint: intermediate representation for Mermaid import/export.
//!
//! A blueprint holds topology (nodes, edges), graph I/O bindings, and metadata. It is the target
//! of the parser and the source for the exporter when not starting from a live `Graph`.

use std::collections::HashMap;

use crate::mermaid::convention;

/// Execution mode for the graph.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExecutionMode {
    /// Nodes may run in any order (default).
    #[default]
    Concurrent,
    /// Nodes run in topological order for reproducibility.
    Deterministic,
}

/// Shard identity when running multiple graph instances (e.g. cluster sharding).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShardConfig {
    /// This instance's shard index (0..total_shards-1).
    pub shard_id: u32,
    /// Total number of shards.
    pub total_shards: u32,
}

/// Per-node supervision policy (from `%% streamweave: node <id> ...`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeSupervision {
    /// Policy: Restart, Stop, Escalate.
    pub policy: String,
    /// Optional group id for shared restart count.
    pub supervision_group: Option<String>,
}

/// One edge in the blueprint: source (node, port) → target (node, port).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlueprintEdge {
    /// Source node id.
    pub source_node: String,
    /// Source output port name.
    pub source_port: String,
    /// Target node id.
    pub target_node: String,
    /// Target input port name.
    pub target_port: String,
}

/// Graph input binding: external name → (node, port).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputBinding {
    /// External input port name.
    pub external_name: String,
    /// Internal node id.
    pub node_id: String,
    /// Input port name on that node.
    pub port_name: String,
}

/// Graph output binding: external name ← (node, port).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputBinding {
    /// External output port name.
    pub external_name: String,
    /// Internal node id.
    pub node_id: String,
    /// Output port name on that node.
    pub port_name: String,
}

/// Optional kind/label for a node (for registry lookup on import).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeInfo {
    /// Optional kind name (e.g. "MapNode") for registry.
    pub kind: Option<String>,
    /// Optional display label.
    pub label: Option<String>,
}

/// Graph blueprint: topology + I/O bindings + metadata.
///
/// This is the canonical in-memory representation produced by the parser and consumed by the
/// exporter (or by `blueprint_to_graph` when building a runnable `Graph`).
#[derive(Clone, Debug, Default)]
pub struct GraphBlueprint {
    /// Graph name.
    pub name: String,
    /// Node ids and optional kind/label.
    pub nodes: HashMap<String, NodeInfo>,
    /// Edges (source_node, source_port, target_node, target_port).
    pub edges: Vec<BlueprintEdge>,
    /// Graph input bindings.
    pub inputs: Vec<InputBinding>,
    /// Graph output bindings.
    pub outputs: Vec<OutputBinding>,
    /// Execution mode (default: concurrent).
    pub execution_mode: ExecutionMode,
    /// Shard config when running multiple instances.
    pub shard_config: Option<ShardConfig>,
    /// Per-node supervision (node_id → policy).
    pub node_supervision: HashMap<String, NodeSupervision>,
    /// Subgraph ids that are supervision units (whole subgraph = one unit).
    pub subgraph_units: Vec<String>,
    /// Edge ids or (source,target) identifiers for feedback edges (round-based execution).
    pub feedback_edge_ids: Vec<String>,
    /// Nested graphs (subgraph id → blueprint). When present, the main blueprint's nodes
    /// include the subgraph id as a graph-as-node; boundary edges use the nested graph's
    /// input/output port names.
    pub subgraphs: HashMap<String, GraphBlueprint>,
}

impl GraphBlueprint {
    /// Creates a new empty blueprint with the given graph name.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    /// Adds a node by id (with optional kind/label).
    pub fn add_node(&mut self, node_id: String, info: NodeInfo) {
        self.nodes.insert(node_id, info);
    }

    /// Adds an edge.
    pub fn add_edge(&mut self, edge: BlueprintEdge) {
        self.edges.push(edge);
    }

    /// Adds a graph input binding.
    pub fn add_input(&mut self, binding: InputBinding) {
        self.inputs.push(binding);
    }

    /// Adds a graph output binding.
    pub fn add_output(&mut self, binding: OutputBinding) {
        self.outputs.push(binding);
    }

    /// Produces a Mermaid flowchart string per the StreamWeave convention (hand-written, no mermaid-builder).
    ///
    /// Emits the `%% streamweave:` comment block (I/O and metadata), then `flowchart TD` with nodes and edges.
    /// Edge labels use the `source_port->target_port` format.
    #[must_use]
    pub fn to_mermaid_string(&self) -> String {
        let mut out = String::new();
        out.push_str("%% streamweave: begin\n");
        for b in &self.inputs {
            out.push_str(&format!(
                "%% streamweave: input {} -> {}.{}\n",
                b.external_name, b.node_id, b.port_name
            ));
        }
        for b in &self.outputs {
            out.push_str(&format!(
                "%% streamweave: output {} <- {}.{}\n",
                b.external_name, b.node_id, b.port_name
            ));
        }
        match self.execution_mode {
            ExecutionMode::Concurrent => {}
            ExecutionMode::Deterministic => {
                out.push_str("%% streamweave: execution_mode=deterministic\n");
            }
        }
        if let Some(s) = &self.shard_config {
            out.push_str(&format!(
                "%% streamweave: shard_config={}/{}\n",
                s.shard_id, s.total_shards
            ));
        }
        for (node_id, sup) in &self.node_supervision {
            let mut line = format!("%% streamweave: node {} supervision_policy={}", node_id, sup.policy);
            if let Some(g) = &sup.supervision_group {
                line.push_str(&format!(" supervision_group={}", g));
            }
            line.push('\n');
            out.push_str(&line);
        }
        for sub_id in &self.subgraph_units {
            out.push_str(&format!("%% streamweave: subgraph_unit {}\n", sub_id));
        }
        for fb in &self.feedback_edge_ids {
            out.push_str(&format!("%% streamweave: feedback {}\n", fb));
        }
        out.push_str("%% streamweave: end\n");
        out.push_str("flowchart TD\n");
        for (node_id, info) in &self.nodes {
            if let Some(label) = &info.label {
                out.push_str(&format!("  {}[\"{}\"]\n", node_id, label.replace('"', "\\\"")));
            } else {
                out.push_str(&format!("  {}\n", node_id));
            }
        }
        for e in &self.edges {
            let label = convention::format_edge_label(&e.source_port, &e.target_port);
            out.push_str(&format!("  {} -->|{}| {}\n", e.source_node, label, e.target_node));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_mermaid_string_contains_comment_block_and_flowchart() {
        let mut bp = GraphBlueprint::new("test".to_string());
        bp.add_node("a".to_string(), NodeInfo::default());
        bp.add_node("b".to_string(), NodeInfo::default());
        bp.add_edge(BlueprintEdge {
            source_node: "a".to_string(),
            source_port: "out".to_string(),
            target_node: "b".to_string(),
            target_port: "in".to_string(),
        });
        let s = bp.to_mermaid_string();
        assert!(s.contains("%% streamweave:"), "must contain comment block");
        assert!(s.contains("flowchart TD"), "must contain flowchart");
        assert!(s.contains("out->in"), "edge label must use convention format");
        assert!(s.contains("a -->|out->in| b"), "edge syntax");
    }

    #[test]
    fn to_mermaid_string_includes_io_bindings_and_metadata() {
        let mut bp = GraphBlueprint::new("pipeline".to_string());
        bp.add_node("src".to_string(), NodeInfo::default());
        bp.add_node("sink".to_string(), NodeInfo::default());
        bp.add_input(InputBinding {
            external_name: "config".to_string(),
            node_id: "src".to_string(),
            port_name: "value".to_string(),
        });
        bp.add_output(OutputBinding {
            external_name: "result".to_string(),
            node_id: "sink".to_string(),
            port_name: "out".to_string(),
        });
        bp.execution_mode = ExecutionMode::Deterministic;
        let s = bp.to_mermaid_string();
        assert!(s.contains("input config -> src.value"));
        assert!(s.contains("output result <- sink.out"));
        assert!(s.contains("execution_mode=deterministic"));
    }
}
