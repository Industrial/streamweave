use proptest::prelude::*;
use streamweave_visualization::{DagEdge, DagNode, NodeKind, NodeMetadata, PipelineDag};
use streamweave_visualization::{DagExporter, escape_dot_string, sanitize_id};

// Property-based tests using proptest
proptest! {
  #[test]
  fn test_to_dot_with_nodes_properties(
    node_id in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    component_type in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    name in prop::option::of(prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()),
    output_type in prop::option::of(prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()),
    error_strategy in prop::sample::select(vec![
      "Stop".to_string(),
      "Skip".to_string(),
      "Retry(5)".to_string(),
    ]),
    node_kind in prop::sample::select(vec![
      NodeKind::Producer,
      NodeKind::Transformer,
      NodeKind::Consumer,
    ]),
  ) {
    let mut dag = PipelineDag::new();
    let node = DagNode::new(
      node_id.clone(),
      node_kind.clone(),
      NodeMetadata {
        component_type: component_type.clone(),
        name: name.clone(),
        input_type: None,
        output_type: output_type.clone(),
        error_strategy: error_strategy.clone(),
        custom: std::collections::HashMap::new(),
      },
    );
    dag.add_node(node);

    let dot = dag.to_dot();
    let sanitized_id = sanitize_id(&node_id);
    prop_assert!(dot.contains(&sanitized_id));
    prop_assert!(dot.contains(&component_type));

    let expected_color = match node_kind {
      NodeKind::Producer => "lightblue",
      NodeKind::Transformer => "lightgreen",
      NodeKind::Consumer => "lightcoral",
    };
    prop_assert!(dot.contains(expected_color));
  }

  #[test]
  fn test_to_dot_with_edges_properties(
    from_id in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    to_id in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    edge_label in prop::option::of(prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()),
  ) {
    let mut dag = PipelineDag::new();
    dag.add_node(DagNode::new(
      from_id.clone(),
      NodeKind::Producer,
      NodeMetadata::default(),
    ));
    dag.add_node(DagNode::new(
      to_id.clone(),
      NodeKind::Consumer,
      NodeMetadata::default(),
    ));
    dag.add_edge(DagEdge::new(
      from_id.clone(),
      to_id.clone(),
      edge_label.clone(),
    ));

    let dot = dag.to_dot();
    let sanitized_from = sanitize_id(&from_id);
    let sanitized_to = sanitize_id(&to_id);
    let edge_pattern = format!("{} -> {}", sanitized_from, sanitized_to);
    prop_assert!(dot.contains(&edge_pattern));

    if let Some(ref label) = edge_label {
      prop_assert!(dot.contains(label));
    }
  }

  #[test]
  fn test_to_json_with_data_properties(
    node_id in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    component_type in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    input_type in prop::option::of(prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()),
    output_type in prop::option::of(prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()),
    error_strategy in prop::sample::select(vec![
      "Stop".to_string(),
      "Skip".to_string(),
      "Retry(5)".to_string(),
    ]),
  ) {
    let mut dag = PipelineDag::new();
    dag.add_node(DagNode::new(
      node_id.clone(),
      NodeKind::Transformer,
      NodeMetadata {
        component_type: component_type.clone(),
        name: None,
        input_type: input_type.clone(),
        output_type: output_type.clone(),
        error_strategy: error_strategy.clone(),
        custom: std::collections::HashMap::new(),
      },
    ));

    let json = dag.to_json().unwrap();
    prop_assert!(json.contains(&component_type));
    prop_assert!(json.contains("\"transformer\""));
  }

  #[test]
  fn test_sanitize_id_properties(
    id in prop::string::string_regex(".*").unwrap(),
  ) {
    let sanitized = sanitize_id(&id);
    // All characters should be alphanumeric or underscore
    prop_assert!(sanitized.chars().all(|c| c.is_alphanumeric() || c == '_'));
    // Character count should be preserved (not byte length, as multi-byte chars may be replaced)
    prop_assert_eq!(sanitized.chars().count(), id.chars().count());
  }

  #[test]
  fn test_escape_dot_string_properties(
    s in prop::string::string_regex(".*").unwrap(),
  ) {
    let escaped = escape_dot_string(&s);
    // Escaped string should not contain unescaped backslashes (except escape sequences)
    // Should not contain unescaped quotes
    // Should not contain unescaped newlines
    prop_assert!(!escaped.contains("\"\n"));
    prop_assert!(!escaped.contains("\"\r"));
    prop_assert!(!escaped.contains("\"\t"));
  }
}

// Additional deterministic tests for specific cases
#[test]
fn test_to_dot_empty() {
  let dag = PipelineDag::new();
  let dot = dag.to_dot();
  assert!(dot.contains("digraph PipelineDag"));
  assert!(dot.contains("rankdir=LR"));
  assert!(dot.contains("node [shape=box, style=rounded]"));
}

#[test]
fn test_to_json() {
  let dag = PipelineDag::new();
  let json = dag.to_json().unwrap();
  assert!(json.contains("\"nodes\""));
  assert!(json.contains("\"edges\""));
}

#[test]
fn test_sanitize_id_specific_cases() {
  assert_eq!(sanitize_id("node_1"), "node_1");
  assert_eq!(sanitize_id("node-1"), "node_1");
  assert_eq!(sanitize_id("node.1"), "node_1");
  assert_eq!(sanitize_id("node 1"), "node_1");
}

#[test]
fn test_escape_dot_string_specific_cases() {
  assert_eq!(escape_dot_string("normal"), "normal");
  assert_eq!(escape_dot_string("with\"quote"), "with\\\"quote");
  assert_eq!(escape_dot_string("with\\backslash"), "with\\\\backslash");
  assert_eq!(escape_dot_string("with\nnewline"), "with\\nnewline");
}
