//! # Enhanced Web Visualization Example with Browser Auto-Open
//!
//! This example demonstrates how to generate a standalone HTML file
//! for visualizing pipeline DAGs in a web browser, with automatic
//! browser opening functionality.
//!
//! ## Features
//!
//! - Complex multi-stage pipeline visualization
//! - Standalone HTML generation with embedded DAG data
//! - Cross-platform browser auto-open (Linux/macOS/Windows)
//! - Additional format exports (JSON, DOT)

mod pipeline;

use pipeline::create_complex_pipeline;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use streamweave::visualization::{
  DagEdge, DagExporter, DagNode, NodeKind, NodeMetadata, PipelineDag, generate_standalone_html,
};

/// Creates a DAG from a multi-stage pipeline manually.
fn create_dag_from_multi_stage_pipeline(
  producer: &impl streamweave::Producer<Output = i32>,
  transformer1: &impl streamweave::Transformer<Input = i32, Output = i32>,
  transformer2: &impl streamweave::Transformer<Input = i32, Output = i32>,
  consumer: &impl streamweave::Consumer<Input = i32>,
) -> PipelineDag {
  let mut dag = PipelineDag::new();

  // Create producer node
  let producer_info = producer.component_info();
  let producer_config = producer.config();
  let producer_metadata = NodeMetadata {
    component_type: producer_info.type_name,
    name: producer_config.name(),
    input_type: None,
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", producer_config.error_strategy()),
    custom: std::collections::HashMap::new(),
  };
  let producer_node = DagNode::new(
    "producer".to_string(),
    NodeKind::Producer,
    producer_metadata,
  );
  dag.add_node(producer_node);

  // Create first transformer node
  let transformer1_info = transformer1.component_info();
  let transformer1_config = transformer1.config();
  let transformer1_metadata = NodeMetadata {
    component_type: transformer1_info.type_name,
    name: transformer1_config.name(),
    input_type: Some("i32".to_string()),
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", transformer1_config.error_strategy()),
    custom: std::collections::HashMap::new(),
  };
  let transformer1_node = DagNode::new(
    "transformer1".to_string(),
    NodeKind::Transformer,
    transformer1_metadata,
  );
  dag.add_node(transformer1_node);

  // Create second transformer node
  let transformer2_info = transformer2.component_info();
  let transformer2_config = transformer2.config();
  let transformer2_metadata = NodeMetadata {
    component_type: transformer2_info.type_name,
    name: transformer2_config.name(),
    input_type: Some("i32".to_string()),
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", transformer2_config.error_strategy()),
    custom: std::collections::HashMap::new(),
  };
  let transformer2_node = DagNode::new(
    "transformer2".to_string(),
    NodeKind::Transformer,
    transformer2_metadata,
  );
  dag.add_node(transformer2_node);

  // Create consumer node
  let consumer_info = consumer.component_info();
  let consumer_config = consumer.config();
  let consumer_metadata = NodeMetadata {
    component_type: consumer_info.type_name,
    name: Some(consumer_config.name.clone()),
    input_type: Some("i32".to_string()),
    output_type: None,
    error_strategy: format!("{:?}", consumer_config.error_strategy),
    custom: std::collections::HashMap::new(),
  };
  let consumer_node = DagNode::new(
    "consumer".to_string(),
    NodeKind::Consumer,
    consumer_metadata,
  );
  dag.add_node(consumer_node);

  // Create edges
  dag.add_edge(DagEdge::new(
    "producer".to_string(),
    "transformer1".to_string(),
    Some("i32".to_string()),
  ));
  dag.add_edge(DagEdge::new(
    "transformer1".to_string(),
    "transformer2".to_string(),
    Some("i32".to_string()),
  ));
  dag.add_edge(DagEdge::new(
    "transformer2".to_string(),
    "consumer".to_string(),
    Some("i32".to_string()),
  ));

  dag
}

/// Opens a file in the default web browser (cross-platform).
///
/// This function attempts to open the specified file path in the user's
/// default web browser. It works on Linux, macOS, and Windows.
///
/// # Arguments
///
/// * `path` - The path to the HTML file to open
///
/// # Returns
///
/// `Ok(())` if the browser was opened successfully, or an error if it failed.
///
/// # Platform Support
///
/// - **Linux**: Uses `xdg-open`
/// - **macOS**: Uses `open`
/// - **Windows**: Uses `cmd /c start`
fn open_browser(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
  let path_str = path.to_string_lossy().to_string();

  #[cfg(target_os = "windows")]
  {
    Command::new("cmd")
      .args(["/C", "start", "", &path_str])
      .spawn()?;
  }

  #[cfg(target_os = "macos")]
  {
    Command::new("open").arg(&path_str).spawn()?;
  }

  #[cfg(target_os = "linux")]
  {
    Command::new("xdg-open").arg(&path_str).spawn()?;
  }

  #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
  {
    return Err(format!("Browser auto-open not supported on this platform").into());
  }

  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Enhanced Web Visualization Example");
  println!("===================================================");
  println!();
  println!("This example demonstrates:");
  println!("1. Creating a complex multi-stage pipeline");
  println!("2. Generating a DAG representation");
  println!("3. Generating standalone HTML visualization");
  println!("4. Automatically opening the visualization in your browser");
  println!("5. Exporting additional formats (JSON, DOT)");
  println!();

  // Create complex pipeline components
  let (producer, transformer1, transformer2, consumer) = create_complex_pipeline();

  // Generate DAG representation
  println!("📊 Generating pipeline DAG...");
  let dag =
    create_dag_from_multi_stage_pipeline(&producer, &transformer1, &transformer2, &consumer);

  println!("✅ DAG generated successfully!");
  println!("   Nodes: {}", dag.nodes().len());
  println!("   Edges: {}", dag.edges().len());
  println!();

  // Generate standalone HTML file
  println!("🌐 Generating standalone HTML visualization...");
  let html = generate_standalone_html(&dag);

  // Write to file
  let output_path = PathBuf::from("pipeline_html_visualization.html");
  let mut file = File::create(&output_path)?;
  file.write_all(html.as_bytes())?;

  println!("✅ HTML file generated: {}", output_path.display());
  println!();

  // Try to open browser automatically
  println!("🌍 Attempting to open browser automatically...");
  match open_browser(&output_path) {
    Ok(()) => {
      println!("✅ Browser opened successfully!");
      println!("   The visualization should now be visible in your default browser.");
    }
    Err(e) => {
      println!("⚠️  Could not open browser automatically: {}", e);
      println!(
        "   Please open '{}' manually in your browser.",
        output_path.display()
      );
    }
  }
  println!();

  // Also export JSON and DOT
  println!("📝 Exporting additional formats...");
  let json = dag.to_json()?;
  std::fs::write("pipeline_html_dag.json", json)?;
  println!("   ✓ JSON: pipeline_html_dag.json");

  let dot = dag.to_dot();
  std::fs::write("pipeline_html_dag.dot", dot)?;
  println!("   ✓ DOT: pipeline_html_dag.dot");
  println!();

  println!("✅ Visualization example completed successfully!");
  println!();
  println!("💡 Instructions:");
  println!("   • The HTML file includes an interactive visualization");
  println!("   • Use mouse wheel to zoom in/out");
  println!("   • Click and drag to pan");
  println!("   • Click on nodes to see component details");
  println!("   • Use buttons to reset zoom, fit to screen, or export data");
  println!();
  println!("📋 Generated files:");
  println!(
    "   • {} - Standalone web visualization",
    output_path.display()
  );
  println!("   • pipeline_html_dag.json - DAG data in JSON format");
  println!("   • pipeline_html_dag.dot - DAG data in Graphviz DOT format");
  println!();
  println!("🔧 Troubleshooting:");
  println!("   • If browser didn't open automatically, open the HTML file manually");
  println!("   • On Linux, ensure 'xdg-open' is installed");
  println!("   • On macOS, the 'open' command should be available by default");
  println!("   • On Windows, 'start' command should work automatically");

  Ok(())
}
