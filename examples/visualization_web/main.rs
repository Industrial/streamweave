//! # Pipeline Web Visualization Example
//!
//! This example demonstrates how to generate a standalone HTML file
//! for visualizing pipeline DAGs in a web browser.

mod pipeline;

use pipeline::create_sample_pipeline;
use streamweave::visualization::DagExporter;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use streamweave::visualization::{PipelineDag, generate_standalone_html};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Web Visualization Example");
  println!("=========================================");
  println!();

  // Create sample pipeline components
  let (producer, transformer, consumer) = create_sample_pipeline();

  // Generate DAG representation
  println!("📊 Generating pipeline DAG...");
  let dag = PipelineDag::from_components(&producer, &transformer, &consumer);

  println!("✅ DAG generated successfully!");
  println!("   Nodes: {}", dag.nodes().len());
  println!("   Edges: {}", dag.edges().len());
  println!();

  // Generate standalone HTML file
  println!("🌐 Generating standalone HTML visualization...");
  let html = generate_standalone_html(&dag);

  // Write to file
  let output_path = PathBuf::from("pipeline_visualization.html");
  let mut file = File::create(&output_path)?;
  file.write_all(html.as_bytes())?;

  println!("✅ HTML file generated: {}", output_path.display());
  println!();

  // Also export JSON and DOT
  println!("📝 Exporting additional formats...");
  let json = dag.to_json()?;
  std::fs::write("pipeline_dag.json", json)?;
  println!("   ✓ JSON: pipeline_dag.json");

  let dot = dag.to_dot();
  std::fs::write("pipeline_dag.dot", dot)?;
  println!("   ✓ DOT: pipeline_dag.dot");
  println!();

  println!("✅ Visualization example completed successfully!");
  println!();
  println!("💡 Instructions:");
  println!("   1. Open 'pipeline_visualization.html' in your web browser");
  println!("   2. Use mouse wheel to zoom in/out");
  println!("   3. Click and drag to pan");
  println!("   4. Click on nodes to see details");
  println!("   5. Use buttons to reset zoom, fit to screen, or export data");
  println!();
  println!("📋 Generated files:");
  println!("   • pipeline_visualization.html - Standalone web visualization");
  println!("   • pipeline_dag.json - DAG data in JSON format");
  println!("   • pipeline_dag.dot - DAG data in Graphviz DOT format");

  Ok(())
}
