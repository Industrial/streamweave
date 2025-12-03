//! # Enhanced Console Export Visualization Example
//!
//! This example demonstrates how to generate and export pipeline DAG representations
//! in multiple formats (DOT and JSON) for visualization and debugging.
//!
//! ## Features
//!
//! - Multi-stage pipeline with producer, two transformers, and consumer
//! - DAG generation from pipeline components
//! - Export to DOT format (for Graphviz)
//! - Export to JSON format (for programmatic access)
//! - File export capabilities

mod pipeline;

use pipeline::create_multi_stage_pipeline;
use std::fs::File;
use std::io::Write;
use streamweave::visualization::{
  DagEdge, DagExporter, DagNode, NodeKind, NodeMetadata, PipelineDag,
};

/// Creates a DAG from a multi-stage pipeline manually.
///
/// Since `PipelineDag::from_components` only supports a single transformer,
/// we manually construct the DAG for a multi-stage pipeline.
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Enhanced Console Export Visualization Example");
  println!("============================================================");
  println!();
  println!("This example demonstrates:");
  println!(
    "1. Creating a multi-stage pipeline (producer → transformer1 → transformer2 → consumer)"
  );
  println!("2. Generating a DAG representation");
  println!("3. Exporting to DOT format (for Graphviz)");
  println!("4. Exporting to JSON format");
  println!("5. Saving exports to files");
  println!();

  // Create sample pipeline components
  let (producer, transformer1, transformer2, consumer) = create_multi_stage_pipeline();

  // Generate DAG representation
  println!("📊 Generating pipeline DAG...");
  let dag =
    create_dag_from_multi_stage_pipeline(&producer, &transformer1, &transformer2, &consumer);

  println!("✅ DAG generated successfully!");
  println!("   Nodes: {}", dag.nodes().len());
  println!("   Edges: {}", dag.edges().len());
  println!();

  // Export to DOT format
  println!("📝 Exporting to DOT format (Graphviz)...");
  let dot = dag.to_dot();
  println!("{}", dot);
  println!();

  // Save DOT to file
  let dot_filename = "pipeline_basic.dot";
  let mut dot_file = File::create(dot_filename)?;
  dot_file.write_all(dot.as_bytes())?;
  println!("💾 DOT format saved to: {}", dot_filename);
  println!();

  // Export to JSON format
  println!("📄 Exporting to JSON format...");
  let json = dag.to_json()?;
  println!("{}", json);
  println!();

  // Save JSON to file
  let json_filename = "pipeline_basic.json";
  let mut json_file = File::create(json_filename)?;
  json_file.write_all(json.as_bytes())?;
  println!("💾 JSON format saved to: {}", json_filename);
  println!();

  println!("✅ Visualization example completed successfully!");
  println!();
  println!("💡 Tips:");
  println!("   • Use Graphviz to generate images from DOT file:");
  println!("     dot -Tpng {} -o pipeline_basic.png", dot_filename);
  println!("     dot -Tsvg {} -o pipeline_basic.svg", dot_filename);
  println!("   • Use JSON output for web-based visualization tools");
  println!("   • DAG includes component types, names, and data flow information");
  println!("   • Files are saved in the current working directory");

  Ok(())
}
