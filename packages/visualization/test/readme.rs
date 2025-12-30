//! Integration tests for README examples
//!
//! This file contains tests that verify the code examples in the README.md file
//! work correctly. These tests ensure the documentation stays up-to-date with
//! the actual API.

use std::net::SocketAddr;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;
use streamweave_vec::VecProducer;
use streamweave_visualization::DagExporter;
use streamweave_visualization::PipelineDag;
use streamweave_visualization::VisualizationServer;
use streamweave_visualization::generate_standalone_html;

// Example from README: Generate DAG
#[test]
fn test_readme_generate_dag() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::<i32>::new();

  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  assert_eq!(dag.nodes().len(), 3);
  assert_eq!(dag.edges().len(), 2);
}

// Example from README: Export to DOT
#[test]
fn test_readme_export_to_dot() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::<i32>::new();

  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  let dot = dag.to_dot();

  // Verify DOT format
  assert!(dot.contains("digraph PipelineDag"));
  assert!(dot.contains("rankdir=LR"));
}

// Example from README: Export to JSON
#[test]
fn test_readme_export_to_json() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::<i32>::new();

  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  let json = dag.to_json().unwrap();

  // Verify JSON format
  assert!(json.contains("\"nodes\""));
  assert!(json.contains("\"edges\""));
}

// Example from README: Interactive Visualization
#[test]
fn test_readme_interactive_visualization() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::<i32>::new();

  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  let html = generate_standalone_html(&dag);

  // Verify HTML contains necessary elements
  assert!(html.contains("<!DOCTYPE html") || html.contains("<html"));
  assert!(html.contains("embeddedDagData") || html.contains("dagData"));
}

// Example from README: Pipeline Visualization
#[test]
fn test_readme_pipeline_visualization() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::<i32>::new();

  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  let dot = dag.to_dot();

  // Verify DOT output is generated
  assert!(!dot.is_empty());
  assert!(dot.contains("digraph"));
}

// Example from README: Standalone HTML
#[test]
fn test_readme_standalone_html() {
  let producer = VecProducer::new(vec![1, 2, 3]);
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let consumer = VecConsumer::<i32>::new();

  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  let html = generate_standalone_html(&dag);

  // Verify HTML is generated
  assert!(!html.is_empty());
  assert!(html.len() > 100); // Should have substantial content
}

// Example from README: Visualization Server (basic test)
#[test]
fn test_readme_visualization_server() {
  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let server = VisualizationServer::new(addr);
  assert_eq!(server.address(), addr);
}

#[tokio::test]
async fn test_readme_visualization_server_serve() {
  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let server = VisualizationServer::new(addr);
  let dag = PipelineDag::new();

  // Test that serve_dag works
  let result = server.serve_dag(dag).await;
  assert!(result.is_ok());
}
