//! # DAG Exporters
//!
//! This module provides functionality for exporting pipeline DAGs to various formats
//! for visualization and analysis.
//!
//! ## Supported Formats
//!
//! - **DOT**: Graphviz DOT format for generating visual diagrams
//! - **JSON**: Structured JSON format for programmatic access and web visualization

use crate::dag::PipelineDag;
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
        crate::dag::NodeKind::Producer => "lightblue",
        crate::dag::NodeKind::Transformer => "lightgreen",
        crate::dag::NodeKind::Consumer => "lightcoral",
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
///
/// # Note
///
/// This function is public for testing purposes.
pub fn sanitize_id(id: &str) -> String {
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
///
/// # Note
///
/// This function is public for testing purposes.
pub fn format_node_label(node: &crate::dag::DagNode) -> String {
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
///
/// # Note
///
/// This function is public for testing purposes.
pub fn escape_dot_string(s: &str) -> String {
  s.replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\n', "\\n")
    .replace('\r', "\\r")
    .replace('\t', "\\t")
}
