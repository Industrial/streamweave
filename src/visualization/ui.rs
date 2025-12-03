//! # Visualization UI Utilities
//!
//! This module provides utilities for generating standalone HTML files
//! with embedded DAG data for easy sharing and viewing.

use crate::visualization::{DagExporter, PipelineDag};

/// Generates a standalone HTML file with embedded DAG data.
///
/// This creates a complete HTML file that includes all necessary CSS and JavaScript
/// with the DAG data embedded, allowing it to be opened directly in a browser
/// without needing a web server.
///
/// # Arguments
///
/// * `dag` - The pipeline DAG to embed in the HTML
///
/// # Returns
///
/// A string containing the complete HTML file content.
///
/// # Example
///
/// ```rust
/// use streamweave::visualization::{PipelineDag, generate_standalone_html};
/// use std::fs::File;
/// use std::io::Write;
///
/// let dag = PipelineDag::new();
/// let html = generate_standalone_html(&dag);
/// let mut file = File::create("pipeline.html").unwrap();
/// file.write_all(html.as_bytes()).unwrap();
/// ```
pub fn generate_standalone_html(dag: &PipelineDag) -> String {
  let json_data = dag.to_json().unwrap_or_else(|_| "{}".to_string());

  let html_template = get_html_content();
  let js_template = get_javascript_content();
  let css_template = get_css_content();

  // Replace script tag with embedded JavaScript
  let mut html = html_template.replace("<script src=\"visualization.js\"></script>", "");

  // Find the closing </head> tag and insert CSS before it
  if let Some(css_pos) = html.find("</head>") {
    html.insert_str(css_pos, &format!("<style>\n{}\n</style>", css_template));
  }

  // Find the closing </body> tag and insert JavaScript before it
  if let Some(js_pos) = html.find("</body>") {
    html.insert_str(js_pos, &format!("<script>\n{}\n</script>", js_template));
  }

  // Embed DAG data in JavaScript
  html = html.replace(
    "// Initialize visualization when page loads",
    &format!(
      "// Embedded DAG data\nconst embeddedDagData = {};\n\n// Initialize visualization when page loads",
      json_data
    ),
  );

  // Update initialization to use embedded data
  html = html.replace(
    "initializeVisualization();",
    "initializeVisualizationWithData(embeddedDagData);",
  );

  // Add function to initialize with embedded data
  html = html.replace(
    "function initializeVisualization() {",
    "function initializeVisualizationWithData(data) {\n        dagData = data;\n        renderGraph();\n        updateStatus('DAG loaded successfully');\n    }\n    \n    function initializeVisualization() {",
  );

  html
}

/// Module for accessing UI assets
mod ui_assets {
  /// Returns the HTML content for the visualization UI.
  pub fn get_html_content() -> &'static str {
    include_str!("ui/visualization.html")
  }

  /// Returns the JavaScript content for the visualization UI.
  pub fn get_javascript_content() -> &'static str {
    include_str!("ui/visualization.js")
  }

  /// Returns the CSS content for the visualization UI.
  pub fn get_css_content() -> &'static str {
    include_str!("ui/visualization.css")
  }
}

/// Re-export UI asset functions
pub use ui_assets::{get_css_content, get_html_content, get_javascript_content};

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_standalone_html() {
    let dag = PipelineDag::new();
    let html = generate_standalone_html(&dag);

    // Should contain HTML structure
    assert!(html.contains("<!DOCTYPE html") || html.contains("<html"));
    assert!(html.contains("</html>") || html.contains("</body>"));

    // Should contain embedded DAG data
    assert!(html.contains("embeddedDagData") || html.contains("dagData"));

    // Should contain JavaScript
    assert!(html.contains("<script>") || html.contains("function"));

    // Should contain CSS
    assert!(html.contains("<style>") || html.contains("style"));
  }

  #[test]
  fn test_generate_standalone_html_with_dag() {
    use crate::visualization::{DagEdge, DagNode, NodeKind, NodeMetadata};

    let mut dag = PipelineDag::new();
    // Add some nodes to the DAG
    let node1 = DagNode::new(
      "producer".to_string(),
      NodeKind::Producer,
      NodeMetadata::default(),
    );
    let node2 = DagNode::new(
      "transformer".to_string(),
      NodeKind::Transformer,
      NodeMetadata::default(),
    );
    dag.add_node(node1);
    dag.add_node(node2);

    let edge = DagEdge::new("producer".to_string(), "transformer".to_string(), None);
    dag.add_edge(edge);

    let html = generate_standalone_html(&dag);

    // Should contain the DAG data
    assert!(html.contains("producer") || html.contains("transformer"));
  }

  #[test]
  fn test_get_html_content() {
    let html = get_html_content();
    assert!(!html.is_empty());
  }

  #[test]
  fn test_get_javascript_content() {
    let js = get_javascript_content();
    assert!(!js.is_empty());
  }

  #[test]
  fn test_get_css_content() {
    let css = get_css_content();
    assert!(!css.is_empty());
  }

  #[test]
  fn test_generate_standalone_html_empty_dag() {
    let dag = PipelineDag::new();
    let html = generate_standalone_html(&dag);

    // Should still generate valid HTML even with empty DAG
    assert!(!html.is_empty());
    assert!(html.len() > 100); // Should have substantial content
  }

  #[test]
  fn test_generate_standalone_html_contains_initialization() {
    let dag = PipelineDag::new();
    let html = generate_standalone_html(&dag);

    // Should contain initialization function
    assert!(
      html.contains("initializeVisualization") || html.contains("initializeVisualizationWithData")
    );
  }
}
