//! # Edge Test Suite
//!
//! Comprehensive test suite for the [`Edge`] struct, including construction,
//! field access, and method access.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **Construction**: Creating Edge instances
//! - **Field Access**: Direct field access and method access
//! - **Edge Properties**: Verification of edge connection details
//!
//! ## Test Organization
//!
//! Tests are organized by functionality:
//!
//! - Construction tests
//! - Field access tests
//! - Method access tests
//! - Edge case tests

use crate::graph::edge::Edge;

// ============================================================================
// Construction Tests
// ============================================================================

#[test]
fn test_edge_construction() {
  let edge = Edge {
    source_node: "node1".to_string(),
    source_port: "out".to_string(),
    target_node: "node2".to_string(),
    target_port: "in".to_string(),
  };

  assert_eq!(edge.source_node, "node1");
  assert_eq!(edge.source_port, "out");
  assert_eq!(edge.target_node, "node2");
  assert_eq!(edge.target_port, "in");
}

#[test]
fn test_edge_field_access() {
  let mut edge = Edge {
    source_node: "node1".to_string(),
    source_port: "out".to_string(),
    target_node: "node2".to_string(),
    target_port: "in".to_string(),
  };

  // Direct field access
  assert_eq!(edge.source_node, "node1");
  edge.source_node = "updated".to_string();
  assert_eq!(edge.source_node, "updated");
}

// ============================================================================
// Method Access Tests
// ============================================================================

#[test]
fn test_edge_methods() {
  let edge = Edge {
    source_node: "node1".to_string(),
    source_port: "out".to_string(),
    target_node: "node2".to_string(),
    target_port: "in".to_string(),
  };

  // Test methods
  assert_eq!(edge.source_node(), "node1");
  assert_eq!(edge.source_port(), "out");
  assert_eq!(edge.target_node(), "node2");
  assert_eq!(edge.target_port(), "in");
}

#[test]
fn test_edge_methods_match_fields() {
  let edge = Edge {
    source_node: "source".to_string(),
    source_port: "output".to_string(),
    target_node: "target".to_string(),
    target_port: "input".to_string(),
  };

  // Methods should return the same values as direct field access
  assert_eq!(edge.source_node(), &edge.source_node);
  assert_eq!(edge.source_port(), &edge.source_port);
  assert_eq!(edge.target_node(), &edge.target_node);
  assert_eq!(edge.target_port(), &edge.target_port);
}

#[test]
fn test_edge_all_properties() {
  let edge = Edge {
    source_node: "producer".to_string(),
    source_port: "out_0".to_string(),
    target_node: "transformer".to_string(),
    target_port: "in_1".to_string(),
  };

  assert_eq!(edge.source_node(), "producer");
  assert_eq!(edge.source_port(), "out_0");
  assert_eq!(edge.target_node(), "transformer");
  assert_eq!(edge.target_port(), "in_1");
}

#[test]
fn test_edge_custom_port_names() {
  let edge = Edge {
    source_node: "source".to_string(),
    source_port: "data".to_string(),
    target_node: "sink".to_string(),
    target_port: "input".to_string(),
  };

  assert_eq!(edge.source_port(), "data");
  assert_eq!(edge.target_port(), "input");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_edge_empty_strings() {
  let edge = Edge {
    source_node: "".to_string(),
    source_port: "".to_string(),
    target_node: "".to_string(),
    target_port: "".to_string(),
  };

  assert_eq!(edge.source_node(), "");
  assert_eq!(edge.source_port(), "");
  assert_eq!(edge.target_node(), "");
  assert_eq!(edge.target_port(), "");
}

#[test]
fn test_edge_special_characters() {
  let edge = Edge {
    source_node: "node-1".to_string(),
    source_port: "out_port".to_string(),
    target_node: "node_2".to_string(),
    target_port: "in.port".to_string(),
  };

  assert_eq!(edge.source_node(), "node-1");
  assert_eq!(edge.source_port(), "out_port");
  assert_eq!(edge.target_node(), "node_2");
  assert_eq!(edge.target_port(), "in.port");
}

#[test]
fn test_edge_unicode_names() {
  let edge = Edge {
    source_node: "节点1".to_string(),
    source_port: "输出".to_string(),
    target_node: "节点2".to_string(),
    target_port: "输入".to_string(),
  };

  assert_eq!(edge.source_node(), "节点1");
  assert_eq!(edge.source_port(), "输出");
  assert_eq!(edge.target_node(), "节点2");
  assert_eq!(edge.target_port(), "输入");
}

#[test]
fn test_edge_long_names() {
  let long_name = "a".repeat(1000);
  let edge = Edge {
    source_node: long_name.clone(),
    source_port: "out".to_string(),
    target_node: "target".to_string(),
    target_port: "in".to_string(),
  };

  assert_eq!(edge.source_node().len(), 1000);
  assert_eq!(edge.source_node(), long_name);
}
