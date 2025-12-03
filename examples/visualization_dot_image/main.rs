//! # Graphviz Image Generation Example
//!
//! This example demonstrates how to generate pipeline DAG visualizations
//! as image files (PNG, SVG, PDF, JPG) using Graphviz.
//!
//! ## Features
//!
//! - Graphviz availability detection
//! - Multiple format support (PNG, SVG, PDF, JPG)
//! - Automatic format conversion
//! - Error handling for missing Graphviz

mod pipeline;

use pipeline::create_multi_stage_pipeline;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use streamweave::visualization::{
  DagEdge, DagExporter, DagNode, NodeKind, NodeMetadata, PipelineDag,
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

/// Checks if Graphviz is available on the system.
///
/// # Returns
///
/// `true` if Graphviz `dot` command is available, `false` otherwise.
fn check_graphviz_available() -> bool {
  Command::new("dot").arg("-V").output().is_ok()
}

/// Converts a DOT string to an image file using Graphviz.
///
/// # Arguments
///
/// * `dot_content` - The DOT format string
/// * `output_path` - Path where the image file should be saved
/// * `format` - Image format: "png", "svg", "pdf", or "jpg"
///
/// # Returns
///
/// `Ok(())` if conversion succeeded, or an error if it failed.
fn convert_dot_to_image(
  dot_content: &str,
  output_path: &PathBuf,
  format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  // Validate format
  let valid_formats = ["png", "svg", "pdf", "jpg", "jpeg"];
  if !valid_formats.contains(&format.to_lowercase().as_str()) {
    return Err(
      format!(
        "Unsupported format: {}. Supported: {:?}",
        format, valid_formats
      )
      .into(),
    );
  }

  // Use "jpeg" for Graphviz if "jpg" is requested
  let format_lower = format.to_lowercase();
  let graphviz_format = if format_lower == "jpg" {
    "jpeg"
  } else {
    &format_lower
  };

  // Create a temporary process to pipe DOT content
  let mut child = Command::new("dot")
    .arg("-T")
    .arg(graphviz_format)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()?;

  // Write DOT content to stdin
  if let Some(mut stdin) = child.stdin.take() {
    use std::io::Write;
    stdin.write_all(dot_content.as_bytes())?;
    drop(stdin); // Close stdin
  }

  // Wait for process to complete
  let output = child.wait_with_output()?;

  if !output.status.success() {
    let error_msg = String::from_utf8_lossy(&output.stderr);
    return Err(format!("Graphviz conversion failed: {}", error_msg).into());
  }

  // Write output to file
  let mut file = File::create(output_path)?;
  file.write_all(&output.stdout)?;

  Ok(())
}

/// Generates images in multiple formats from a DAG.
///
/// # Arguments
///
/// * `dag` - The pipeline DAG to visualize
/// * `base_name` - Base name for output files (without extension)
/// * `formats` - Vector of format strings ("png", "svg", "pdf", "jpg")
///
/// # Returns
///
/// A vector of tuples `(format, path, success)` indicating which formats were generated.
fn generate_images(
  dag: &PipelineDag,
  base_name: &str,
  formats: &[&str],
) -> Vec<(String, PathBuf, bool)> {
  let dot = dag.to_dot();
  let mut results = Vec::new();

  for format in formats {
    let output_path = PathBuf::from(format!("{}.{}", base_name, format));
    match convert_dot_to_image(&dot, &output_path, format) {
      Ok(()) => {
        println!("   ✓ {}: {}", format.to_uppercase(), output_path.display());
        results.push((format.to_string(), output_path, true));
      }
      Err(e) => {
        println!("   ✗ {}: Failed - {}", format.to_uppercase(), e);
        results.push((format.to_string(), output_path, false));
      }
    }
  }

  results
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Graphviz Image Generation Example");
  println!("=================================================");
  println!();

  // Check Graphviz availability
  println!("🔍 Checking Graphviz availability...");
  if !check_graphviz_available() {
    println!("❌ Graphviz is not installed or not in PATH");
    println!();
    println!("📦 Installation Instructions:");
    println!();
    println!("Linux (Ubuntu/Debian):");
    println!("  sudo apt-get install graphviz");
    println!();
    println!("Linux (Fedora):");
    println!("  sudo dnf install graphviz");
    println!();
    println!("macOS:");
    println!("  brew install graphviz");
    println!();
    println!("Windows:");
    println!("  Download from https://graphviz.org/download/");
    println!("  Add Graphviz bin directory to PATH");
    println!();
    return Err("Graphviz not available".into());
  }
  println!("✅ Graphviz is available!");
  println!();

  // Create pipeline components
  let (producer, transformer1, transformer2, consumer) = create_multi_stage_pipeline();

  // Generate DAG representation
  println!("📊 Generating pipeline DAG...");
  let dag =
    create_dag_from_multi_stage_pipeline(&producer, &transformer1, &transformer2, &consumer);

  println!("✅ DAG generated successfully!");
  println!("   Nodes: {}", dag.nodes().len());
  println!("   Edges: {}", dag.edges().len());
  println!();

  // Save DOT file
  println!("📝 Saving DOT file...");
  let dot = dag.to_dot();
  let dot_path = PathBuf::from("pipeline_dot_image.dot");
  let mut dot_file = File::create(&dot_path)?;
  dot_file.write_all(dot.as_bytes())?;
  println!("✅ DOT file saved: {}", dot_path.display());
  println!();

  // Generate images in multiple formats
  println!("🖼️  Generating images in multiple formats...");
  let formats = vec!["png", "svg", "pdf", "jpg"];
  let results = generate_images(&dag, "pipeline_dot_image", &formats);
  println!();

  // Summary
  let success_count = results.iter().filter(|(_, _, success)| *success).count();
  let total_count = results.len();

  println!("✅ Image generation completed!");
  println!(
    "   Successfully generated: {}/{} formats",
    success_count, total_count
  );
  println!();

  if success_count > 0 {
    println!("📋 Generated files:");
    for (format, path, success) in &results {
      if *success {
        println!("   • {} - {}", path.display(), format.to_uppercase());
      }
    }
    println!();
  }

  if success_count < total_count {
    println!("⚠️  Some formats failed to generate. Check error messages above.");
    println!();
  }

  println!("💡 Tips:");
  println!("   • PNG: Best for presentations and web use");
  println!("   • SVG: Scalable vector format, great for web");
  println!("   • PDF: Best for documents and printing");
  println!("   • JPG: Smaller file size, good for sharing");
  println!();
  println!("   • Use the DOT file to regenerate images with custom Graphviz options:");
  println!("     dot -Tpng pipeline_dot_image.dot -o custom.png");
  println!("     dot -Tsvg pipeline_dot_image.dot -o custom.svg");

  Ok(())
}
