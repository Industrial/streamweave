//! # Debug Mode Visualization Example
//!
//! This example demonstrates how to create debug-mode visualizations with
//! breakpoint support for StreamWeave pipelines.
//!
//! ## Features
//!
//! - Breakpoint setup and configuration
//! - Debug visualization with enhanced DAG metadata
//! - HTML generation with debug UI
//! - Breakpoint configuration JSON export

mod pipeline;

use pipeline::create_multi_stage_pipeline;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use streamweave::visualization::debug::{Breakpoint, Debugger};
use streamweave::visualization::{
  DagEdge, DagExporter, DagNode, NodeKind, NodeMetadata, PipelineDag, generate_standalone_html,
};

/// Creates a DAG from a multi-stage pipeline with debug metadata.
fn create_dag_with_debug_metadata(
  producer: &impl streamweave::Producer<Output = i32>,
  transformer1: &impl streamweave::Transformer<Input = i32, Output = i32>,
  transformer2: &impl streamweave::Transformer<Input = i32, Output = i32>,
  consumer: &impl streamweave::Consumer<Input = i32>,
  _debugger: &Debugger,
) -> PipelineDag {
  let mut dag = PipelineDag::new();

  // Helper to create node with debug metadata
  let create_node_with_debug = |id: &str, kind: NodeKind, metadata: NodeMetadata| {
    let mut node = DagNode::new(id.to_string(), kind, metadata);

    // Add debug metadata to custom fields
    // In a real implementation, this would check debugger state
    node
      .metadata
      .custom
      .insert("debug_enabled".to_string(), "true".to_string());

    node
  };

  // Create producer node
  let producer_info = producer.component_info();
  let producer_config = producer.config();
  let mut producer_metadata = NodeMetadata {
    component_type: producer_info.type_name,
    name: producer_config.name(),
    input_type: None,
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", producer_config.error_strategy()),
    custom: HashMap::new(),
  };
  producer_metadata
    .custom
    .insert("debug_enabled".to_string(), "true".to_string());
  let producer_node = create_node_with_debug("producer", NodeKind::Producer, producer_metadata);
  dag.add_node(producer_node);

  // Create first transformer node
  let transformer1_info = transformer1.component_info();
  let transformer1_config = transformer1.config();
  let mut transformer1_metadata = NodeMetadata {
    component_type: transformer1_info.type_name,
    name: transformer1_config.name(),
    input_type: Some("i32".to_string()),
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", transformer1_config.error_strategy()),
    custom: HashMap::new(),
  };
  transformer1_metadata
    .custom
    .insert("debug_enabled".to_string(), "true".to_string());
  transformer1_metadata
    .custom
    .insert("has_breakpoint".to_string(), "true".to_string());
  let transformer1_node =
    create_node_with_debug("transformer1", NodeKind::Transformer, transformer1_metadata);
  dag.add_node(transformer1_node);

  // Create second transformer node
  let transformer2_info = transformer2.component_info();
  let transformer2_config = transformer2.config();
  let mut transformer2_metadata = NodeMetadata {
    component_type: transformer2_info.type_name,
    name: transformer2_config.name(),
    input_type: Some("i32".to_string()),
    output_type: Some("i32".to_string()),
    error_strategy: format!("{:?}", transformer2_config.error_strategy()),
    custom: HashMap::new(),
  };
  transformer2_metadata
    .custom
    .insert("debug_enabled".to_string(), "true".to_string());
  let transformer2_node =
    create_node_with_debug("transformer2", NodeKind::Transformer, transformer2_metadata);
  dag.add_node(transformer2_node);

  // Create consumer node
  let consumer_info = consumer.component_info();
  let consumer_config = consumer.config();
  let mut consumer_metadata = NodeMetadata {
    component_type: consumer_info.type_name,
    name: Some(consumer_config.name.clone()),
    input_type: Some("i32".to_string()),
    output_type: None,
    error_strategy: format!("{:?}", consumer_config.error_strategy),
    custom: HashMap::new(),
  };
  consumer_metadata
    .custom
    .insert("debug_enabled".to_string(), "true".to_string());
  let consumer_node = create_node_with_debug("consumer", NodeKind::Consumer, consumer_metadata);
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

/// Enhances HTML with debug UI and breakpoint information.
fn enhance_html_with_debug(html: &str, _debugger: &Debugger, breakpoints: &[Breakpoint]) -> String {
  // Convert breakpoints to JSON
  let breakpoints_json = serde_json::to_string(breakpoints).unwrap_or_else(|_| "[]".to_string());

  // Get debug state
  let state_json = {
    // In a real async context, we'd await this
    // For this example, we'll use a default state
    r#""Running""#
  };

  // Find the closing </body> tag and insert debug UI before it
  if let Some(body_pos) = html.rfind("</body>") {
    let mut enhanced = html.to_string();

    // Insert debug UI and script
    let debug_html = format!(
      r#"
    <script>
      // Embedded debug configuration
      const debugConfig = {{
        breakpoints: {},
        state: {},
        debugMode: true
      }};
      
      // Debug UI rendering
      function renderDebugUI() {{
        const debugContainer = document.getElementById('debug-container');
        if (!debugContainer) {{
          return;
        }}
        
        let html = '<div style="margin: 20px; padding: 20px; background: #f0f0f0; border-radius: 8px;">';
        html += '<h2 style="margin-top: 0;">Debug Mode</h2>';
        
        // Debug state
        html += '<div style="margin-bottom: 15px; padding: 10px; background: white; border-radius: 4px;">';
        html += '<strong>Debug State:</strong> <span id="debug-state">' + debugConfig.state + '</span>';
        html += '</div>';
        
        // Breakpoints list
        html += '<div style="margin-bottom: 15px; padding: 10px; background: white; border-radius: 4px;">';
        html += '<h3 style="margin-top: 0;">Breakpoints (' + debugConfig.breakpoints.length + ')</h3>';
        html += '<ul style="margin: 0; padding-left: 20px;">';
        debugConfig.breakpoints.forEach(bp => {{
          const status = bp.enabled ? '✓ Enabled' : '✗ Disabled';
          const condition = bp.condition ? ' (condition: ' + escapeHtml(bp.condition) + ')' : '';
          html += '<li>' + escapeHtml(bp.node_id) + ' - ' + status + condition + '</li>';
        }});
        html += '</ul>';
        html += '</div>';
        
        // Debug controls
        html += '<div style="padding: 10px; background: white; border-radius: 4px;">';
        html += '<h3 style="margin-top: 0;">Debug Controls</h3>';
        html += '<div style="display: flex; gap: 10px; flex-wrap: wrap;">';
        html += '<button onclick="debugResume()" style="padding: 8px 16px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;">Resume</button>';
        html += '<button onclick="debugStep()" style="padding: 8px 16px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer;">Step</button>';
        html += '<button onclick="debugPause()" style="padding: 8px 16px; background: #FF9800; color: white; border: none; border-radius: 4px; cursor: pointer;">Pause</button>';
        html += '</div>';
        html += '</div>';
        
        html += '</div>';
        debugContainer.innerHTML = html;
      }}
      
      function escapeHtml(text) {{
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
      }}
      
      function debugResume() {{
        document.getElementById('debug-state').textContent = 'Running';
        updateStatus('Debug: Resumed execution');
      }}
      
      function debugStep() {{
        document.getElementById('debug-state').textContent = 'Stepping';
        updateStatus('Debug: Stepping through execution');
      }}
      
      function debugPause() {{
        document.getElementById('debug-state').textContent = 'Paused';
        updateStatus('Debug: Execution paused');
      }}
      
      // Render debug UI when page loads
      if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', renderDebugUI);
      }} else {{
        renderDebugUI();
      }}
    </script>
    
    <div id="debug-container"></div>
"#,
      breakpoints_json, state_json
    );

    enhanced.insert_str(body_pos, &debug_html);
    enhanced
  } else {
    html.to_string()
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Debug Mode Visualization Example");
  println!("===============================================");
  println!();
  println!("This example demonstrates:");
  println!("1. Setting up breakpoints for debugging");
  println!("2. Creating debug visualization with enhanced DAG metadata");
  println!("3. Generating HTML with debug UI");
  println!("4. Exporting breakpoint configuration");
  println!();

  // Create pipeline components
  let (producer, transformer1, transformer2, consumer) = create_multi_stage_pipeline();

  // Initialize debugger
  println!("🐛 Initializing debugger...");
  let mut debugger = Debugger::new();
  println!("✅ Debugger initialized!");
  println!();

  // Set up breakpoints
  println!("📍 Setting up breakpoints...");
  let breakpoint1 = Breakpoint::at_node("transformer1".to_string());
  let breakpoint2 = Breakpoint::with_condition("transformer2".to_string(), "item > 10".to_string());

  debugger.add_breakpoint(breakpoint1.clone()).await;
  debugger.add_breakpoint(breakpoint2.clone()).await;

  let breakpoints = vec![breakpoint1, breakpoint2];
  println!("✅ Breakpoints configured!");
  println!("   Breakpoint 1: transformer1 (always)");
  println!("   Breakpoint 2: transformer2 (condition: item > 10)");
  println!();

  // Generate DAG with debug metadata
  println!("📊 Generating DAG with debug metadata...");
  let dag = create_dag_with_debug_metadata(
    &producer,
    &transformer1,
    &transformer2,
    &consumer,
    &debugger,
  );

  println!("✅ DAG generated successfully!");
  println!("   Nodes: {}", dag.nodes().len());
  println!("   Edges: {}", dag.edges().len());
  println!("   Debug metadata: Enabled");
  println!();

  // Generate base HTML
  println!("🌐 Generating HTML visualization with debug UI...");
  let base_html = generate_standalone_html(&dag);

  // Enhance HTML with debug UI
  let enhanced_html = enhance_html_with_debug(&base_html, &debugger, &breakpoints);

  // Write enhanced HTML to file
  let output_path = PathBuf::from("pipeline_debug_visualization.html");
  let mut file = File::create(&output_path)?;
  file.write_all(enhanced_html.as_bytes())?;

  println!("✅ Enhanced HTML file generated: {}", output_path.display());
  println!();

  // Export breakpoint configuration
  println!("📝 Exporting breakpoint configuration...");
  let breakpoints_json = serde_json::to_string_pretty(&breakpoints)?;
  std::fs::write("pipeline_debug_breakpoints.json", &breakpoints_json)?;
  println!("✅ Breakpoint configuration exported to: pipeline_debug_breakpoints.json");
  println!();

  // Export debug state
  println!("📝 Exporting debug state...");
  let state = debugger.state.read().await;
  let state_json = serde_json::to_string_pretty(&*state)?;
  drop(state);
  std::fs::write("pipeline_debug_state.json", &state_json)?;
  println!("✅ Debug state exported to: pipeline_debug_state.json");
  println!();

  // Also export DAG formats
  println!("📝 Exporting additional formats...");
  let dag_json = dag.to_json()?;
  std::fs::write("pipeline_debug_dag.json", dag_json)?;
  println!("   ✓ DAG JSON: pipeline_debug_dag.json");

  let dag_dot = dag.to_dot();
  std::fs::write("pipeline_debug_dag.dot", dag_dot)?;
  println!("   ✓ DAG DOT: pipeline_debug_dag.dot");
  println!();

  println!("✅ Debug visualization example completed successfully!");
  println!();
  println!("📋 Generated files:");
  println!(
    "   • {} - Interactive visualization with debug UI",
    output_path.display()
  );
  println!("   • pipeline_debug_breakpoints.json - Breakpoint configuration");
  println!("   • pipeline_debug_state.json - Current debug state");
  println!("   • pipeline_debug_dag.json - DAG structure with debug metadata");
  println!("   • pipeline_debug_dag.dot - DAG structure in DOT format");
  println!();
  println!("💡 Debug Features:");
  println!("   • Breakpoint configuration and management");
  println!("   • Debug state tracking (Running, Paused, Stepping)");
  println!("   • Enhanced DAG metadata for debug visualization");
  println!("   • Interactive debug UI in HTML visualization");
  println!("   • Breakpoint configuration export");
  println!();
  println!("💡 Debug Workflow:");
  println!("   1. Set breakpoints at nodes where you want to pause");
  println!("   2. Run pipeline - execution pauses at breakpoints");
  println!("   3. Inspect data snapshots at pause points");
  println!("   4. Use debug controls (Resume, Step, Pause)");
  println!("   5. Export breakpoint configuration for reuse");
  println!();
  println!("💡 Instructions:");
  println!("   • Open the HTML file in your browser to view the debug visualization");
  println!("   • Debug UI is displayed below the DAG visualization");
  println!("   • Use breakpoint JSON to configure breakpoints programmatically");

  Ok(())
}
