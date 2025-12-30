use streamweave_visualization::{DagEdge, DagNode, NodeKind, NodeMetadata, PipelineDag};
use streamweave_visualization::{
  generate_standalone_html, get_css_content, get_html_content, get_javascript_content,
};

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
