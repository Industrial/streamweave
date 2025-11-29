//! # DAG Exporters
//!
//! This module provides functionality for exporting pipeline DAGs to various formats
//! for visualization and analysis.
//!
//! ## Supported Formats
//!
//! - **DOT**: Graphviz DOT format for generating visual diagrams
//! - **JSON**: Structured JSON format for programmatic access and web visualization

use crate::visualization::PipelineDag;
use serde_json;
use std::fmt::Write;

/// Trait for exporting pipeline DAGs to different formats.
///
/// Implementations of this trait can convert a `PipelineDag` into various
/// serialization formats for visualization or analysis.
///
/// # Example
///
/// ```rust
/// use streamweave::visualization::{PipelineDag, DagExporter};
///
/// let dag = PipelineDag::new();
/// let dot_string = dag.to_dot();
/// let json_string = dag.to_json().unwrap();
/// ```
pub trait DagExporter {
  /// Exports the DAG to DOT (Graphviz) format.
  ///
  /// # Returns
  ///
  /// A string containing the DOT representation of the DAG.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::{PipelineDag, DagExporter};
  ///
  /// let dag = PipelineDag::new();
  /// let dot = dag.to_dot();
  /// // Can be used with Graphviz: `dot -Tpng <<< "$dot"`
  /// ```
  fn to_dot(&self) -> String;

  /// Exports the DAG to JSON format.
  ///
  /// # Returns
  ///
  /// A `Result` containing either the JSON string representation or a serialization error.
  ///
  /// # Errors
  ///
  /// Returns an error if JSON serialization fails.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::{PipelineDag, DagExporter};
  ///
  /// let dag = PipelineDag::new();
  /// match dag.to_json() {
  ///     Ok(json) => println!("{}", json),
  ///     Err(e) => eprintln!("Failed to serialize: {}", e),
  /// }
  /// ```
  fn to_json(&self) -> Result<String, serde_json::Error>;
}

impl DagExporter for PipelineDag {
  fn to_dot(&self) -> String {
    let mut output = String::new();

    // Write graph header
    writeln!(output, "digraph PipelineDag {{").unwrap();
    writeln!(output, "  rankdir=LR;").unwrap();
    writeln!(output, "  node [shape=box, style=rounded];").unwrap();

    // Write nodes
    for node in &self.nodes {
      let node_id = sanitize_id(&node.id);
      let label = format_node_label(node);
      let color = match node.kind {
        crate::visualization::NodeKind::Producer => "lightblue",
        crate::visualization::NodeKind::Transformer => "lightgreen",
        crate::visualization::NodeKind::Consumer => "lightcoral",
      };

      writeln!(
        output,
        "  {} [label=\"{}\", fillcolor={}, style=\"rounded,filled\"];",
        node_id, label, color
      )
      .unwrap();
    }

    // Write edges
    for edge in &self.edges {
      let from_id = sanitize_id(&edge.from);
      let to_id = sanitize_id(&edge.to);
      let label_attr = if let Some(ref label) = edge.label {
        format!(" [label=\"{}\"]", escape_dot_string(label))
      } else {
        String::new()
      };

      writeln!(output, "  {} -> {}{};", from_id, to_id, label_attr).unwrap();
    }

    writeln!(output, "}}").unwrap();
    output
  }

  fn to_json(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(self)
  }
}

/// Sanitizes a string to be a valid DOT identifier.
///
/// DOT identifiers can only contain alphanumeric characters and underscores.
/// This function replaces invalid characters with underscores.
///
/// # Arguments
///
/// * `id` - The identifier to sanitize
///
/// # Returns
///
/// A sanitized identifier safe for use in DOT format.
fn sanitize_id(id: &str) -> String {
  id.chars()
    .map(|c| {
      if c.is_alphanumeric() || c == '_' {
        c
      } else {
        '_'
      }
    })
    .collect()
}

/// Formats a node label for DOT output.
///
/// Creates a multi-line label with node information including the component type
/// and name (if available).
///
/// # Arguments
///
/// * `node` - The node to format
///
/// # Returns
///
/// A formatted label string.
fn format_node_label(node: &crate::visualization::DagNode) -> String {
  let mut label = String::new();

  // Add component type
  label.push_str(&escape_dot_string(&node.metadata.component_type));

  // Add name if available
  if let Some(ref name) = node.metadata.name {
    label.push_str("\\n");
    label.push_str(&escape_dot_string(name));
  }

  // Add output type if available
  if let Some(ref output_type) = node.metadata.output_type {
    label.push_str("\\n");
    label.push_str("→ ");
    label.push_str(&escape_dot_string(output_type));
  }

  label
}

/// Escapes special characters in a string for use in DOT format.
///
/// DOT format requires certain characters to be escaped. This function handles
/// common cases like quotes and newlines.
///
/// # Arguments
///
/// * `s` - The string to escape
///
/// # Returns
///
/// An escaped string safe for use in DOT labels.
fn escape_dot_string(s: &str) -> String {
  s.replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\n', "\\n")
    .replace('\r', "\\r")
    .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::visualization::{DagEdge, DagNode, NodeKind, NodeMetadata};
  use proptest::prelude::*;

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
}
