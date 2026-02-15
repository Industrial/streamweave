//! Build a runnable [`Graph`](crate::graph::Graph) from a [blueprint](crate::mermaid::blueprint::GraphBlueprint).
//!
//! When no node registry is supplied, every blueprint node is instantiated as a
//! [placeholder](crate::mermaid::placeholder_node::PlaceholderNode) so the graph structure
//! can be validated and exported; execution is a no-op. When a [`NodeRegistry`] is supplied,
//! nodes with a `kind` in the blueprint (e.g. from `%% streamweave: node <id> kind=...`) are
//! instantiated via the registry if the kind is registered; others remain placeholders.

use crate::edge::Edge;
use crate::graph::{Graph, GraphExecutionError};
use crate::mermaid::blueprint::GraphBlueprint;
use crate::mermaid::placeholder_node::PlaceholderNode;
use crate::node::Node;
use std::collections::{HashMap, HashSet};

/// Type alias for node constructor: `(node_id, input_ports, output_ports) -> Box<dyn Node>`.
type NodeConstructor =
  Box<dyn Fn(String, Vec<String>, Vec<String>) -> Box<dyn Node> + Send + Sync + 'static>;

/// Registry that maps node **kind** (e.g. from `%% streamweave: node <id> kind=MapNode`) to a
/// constructor that builds a `Box<dyn Node>` with the given node id and port names.
///
/// Use with [`blueprint_to_graph`] so that parsed blueprints can be turned into graphs with
/// real nodes instead of placeholders.
#[derive(Default)]
pub struct NodeRegistry {
  /// Maps kind name to constructor `(node_id, input_ports, output_ports) -> Box<dyn Node>`.
  constructors: HashMap<String, NodeConstructor>,
}

impl NodeRegistry {
  /// Creates an empty registry.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Registers a constructor for the given kind. The constructor receives `(node_id, input_port_names, output_port_names)`.
  pub fn register<K, F>(&mut self, kind: K, ctor: F)
  where
    K: Into<String>,
    F: Fn(String, Vec<String>, Vec<String>) -> Box<dyn Node> + Send + Sync + 'static,
  {
    self.constructors.insert(kind.into(), Box::new(ctor));
  }

  /// Returns a constructor for the kind if registered.
  #[must_use]
  pub fn get(&self, kind: &str) -> Option<&NodeConstructor> {
    self.constructors.get(kind)
  }
}

/// Builds a `Graph` from a blueprint.
///
/// If `registry` is `None`, every node is a [`PlaceholderNode`] (structure/roundtrip only).
/// If `registry` is `Some`, each node's **kind** (from [`NodeInfo::kind`](crate::mermaid::blueprint::NodeInfo)) is looked up; when the kind is registered, the constructor is called with `(node_id, input_ports, output_ports)` and the returned node is used; otherwise a placeholder is used.
///
/// Graph I/O bindings, execution mode, and shard config are applied. Nested blueprints
/// (subgraphs) are not expanded; the subgraph id remains a placeholder node.
///
/// # Errors
///
/// Returns an error if adding a node or edge fails (e.g. duplicate node, invalid port).
pub fn blueprint_to_graph(
  bp: &GraphBlueprint,
  registry: Option<&NodeRegistry>,
) -> Result<Graph, GraphExecutionError> {
  let mut graph = Graph::new(bp.name.clone());

  let (input_ports, output_ports) = ports_per_node(bp);

  for (node_id, info) in &bp.nodes {
    let inputs = input_ports.get(node_id).cloned().unwrap_or_default();
    let outputs = output_ports.get(node_id).cloned().unwrap_or_default();
    let node: Box<dyn Node> =
      match registry.and_then(|reg| info.kind.as_deref().and_then(|k| reg.get(k))) {
        Some(ctor) => ctor(node_id.clone(), inputs.clone(), outputs.clone()),
        None => Box::new(PlaceholderNode::new(node_id.clone(), inputs, outputs)),
      };
    graph
      .add_node(node_id.clone(), node)
      .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::other(e)) })?;
  }

  for e in &bp.edges {
    graph
      .add_edge(Edge {
        source_node: e.source_node.clone(),
        source_port: e.source_port.clone(),
        target_node: e.target_node.clone(),
        target_port: e.target_port.clone(),
      })
      .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::other(e)) })?;
  }

  for b in &bp.inputs {
    graph
      .expose_input_port(&b.node_id, &b.port_name, &b.external_name)
      .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::other(e)) })?;
  }
  for b in &bp.outputs {
    graph
      .expose_output_port(&b.node_id, &b.port_name, &b.external_name)
      .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::other(e)) })?;
  }

  graph.set_execution_mode(match bp.execution_mode {
    crate::mermaid::blueprint::ExecutionMode::Concurrent => crate::graph::ExecutionMode::Concurrent,
    crate::mermaid::blueprint::ExecutionMode::Deterministic => {
      crate::graph::ExecutionMode::Deterministic
    }
  });
  if let Some(s) = &bp.shard_config {
    graph.set_shard_config(s.shard_id, s.total_shards);
  }
  for (node_id, sup) in &bp.node_supervision {
    use crate::supervision::{FailureAction, SupervisionPolicy};
    let on_failure = match sup.policy.as_str() {
      "Restart" | "restart" => FailureAction::Restart,
      "RestartGroup" | "restart_group" => FailureAction::RestartGroup,
      "Stop" | "stop" => FailureAction::Stop,
      "Escalate" | "escalate" => FailureAction::Escalate,
      _ => FailureAction::Restart,
    };
    graph.set_node_supervision_policy(
      node_id,
      SupervisionPolicy::new(on_failure).with_max_restarts(Some(3)),
    );
    if let Some(ref g) = sup.supervision_group {
      graph.set_supervision_group(node_id, g);
    }
  }

  Ok(graph)
}

/// Computes input and output port names per node from blueprint edges and I/O bindings.
fn ports_per_node(
  bp: &GraphBlueprint,
) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
  let mut input_ports: HashMap<String, HashSet<String>> = HashMap::new();
  let mut output_ports: HashMap<String, HashSet<String>> = HashMap::new();
  for e in &bp.edges {
    input_ports
      .entry(e.target_node.clone())
      .or_default()
      .insert(e.target_port.clone());
    output_ports
      .entry(e.source_node.clone())
      .or_default()
      .insert(e.source_port.clone());
  }
  for b in &bp.inputs {
    input_ports
      .entry(b.node_id.clone())
      .or_default()
      .insert(b.port_name.clone());
  }
  for b in &bp.outputs {
    output_ports
      .entry(b.node_id.clone())
      .or_default()
      .insert(b.port_name.clone());
  }
  let to_vec = |m: HashMap<String, HashSet<String>>| {
    m.into_iter()
      .map(|(k, v)| (k, v.into_iter().collect::<Vec<_>>()))
      .collect()
  };
  (to_vec(input_ports), to_vec(output_ports))
}

#[cfg(test)]
mod tests {
  use super::{NodeRegistry, blueprint_to_graph};
  use crate::mermaid::blueprint::{
    BlueprintEdge, ExecutionMode, GraphBlueprint, InputBinding, NodeInfo, OutputBinding,
  };
  use crate::mermaid::export::graph_to_mermaid;
  use crate::mermaid::placeholder_node::PlaceholderNode;

  #[test]
  fn blueprint_to_graph_builds_graph_with_placeholders() {
    let mut bp = GraphBlueprint::new("test_linear".to_string());
    bp.add_node("A".to_string(), NodeInfo::default());
    bp.add_node("B".to_string(), NodeInfo::default());
    bp.add_edge(BlueprintEdge {
      source_node: "A".to_string(),
      source_port: "out".to_string(),
      target_node: "B".to_string(),
      target_port: "in".to_string(),
    });
    bp.add_input(InputBinding {
      external_name: "in".to_string(),
      node_id: "A".to_string(),
      port_name: "in".to_string(),
    });
    bp.add_output(OutputBinding {
      external_name: "out".to_string(),
      node_id: "B".to_string(),
      port_name: "out".to_string(),
    });
    bp.execution_mode = ExecutionMode::Deterministic;

    let graph = blueprint_to_graph(&bp, None).expect("blueprint_to_graph");
    let mmd = graph_to_mermaid(&graph);
    assert!(
      mmd.contains("streamweave"),
      "export should contain streamweave block"
    );
    assert!(mmd.contains("test_linear") || mmd.contains("flowchart") || mmd.contains("graph"));
    assert!(mmd.contains("A") && mmd.contains("B"));
    assert!(mmd.contains("out->in") || mmd.contains("input") || mmd.contains("output"));
  }

  #[test]
  fn blueprint_to_graph_with_registry_uses_constructor_for_kind() {
    let mut bp = GraphBlueprint::new("with_registry".to_string());
    let info = NodeInfo {
      kind: Some("Custom".to_string()),
      ..Default::default()
    };
    bp.add_node("n1".to_string(), info);
    bp.add_node("n2".to_string(), NodeInfo::default());
    bp.add_edge(BlueprintEdge {
      source_node: "n1".to_string(),
      source_port: "out".to_string(),
      target_node: "n2".to_string(),
      target_port: "in".to_string(),
    });
    let mut reg = NodeRegistry::new();
    reg.register("Custom", |name, inputs, outputs| {
      Box::new(PlaceholderNode::new(name, inputs, outputs))
    });
    let graph = blueprint_to_graph(&bp, Some(&reg)).expect("blueprint_to_graph");
    let mmd = graph_to_mermaid(&graph);
    assert!(mmd.contains("n1") && mmd.contains("n2"));
  }
}
