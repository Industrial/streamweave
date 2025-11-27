//! # Pipeline Visualization Example
//!
//! This example demonstrates how to generate and export pipeline DAG representations
//! for visualization and debugging.

mod pipeline;

use pipeline::create_sample_pipeline;
use streamweave::visualization::{DagExporter, PipelineDag};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Pipeline Visualization Example");
  println!("=============================================");
  println!();
  println!("This example demonstrates:");
  println!("1. Creating a pipeline");
  println!("2. Generating a DAG representation");
  println!("3. Exporting to DOT format (for Graphviz)");
  println!("4. Exporting to JSON format");
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

  // Export to DOT format
  println!("📝 Exporting to DOT format (Graphviz)...");
  let dot = dag.to_dot();
  println!("{}", dot);
  println!();

  // Export to JSON format
  println!("📄 Exporting to JSON format...");
  let json = dag.to_json()?;
  println!("{}", json);
  println!();

  println!("✅ Visualization example completed successfully!");
  println!();
  println!("💡 Tips:");
  println!("   • Save DOT output to a file and use Graphviz to generate images:");
  println!("     dot -Tpng pipeline.dot -o pipeline.png");
  println!("   • Use JSON output for web-based visualization tools");
  println!("   • DAG includes component types, names, and data flow information");

  Ok(())
}
