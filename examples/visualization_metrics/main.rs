//! # Real-Time Metrics Visualization Example
//!
//! This example demonstrates how to collect and visualize real-time metrics
//! from a StreamWeave pipeline, including throughput, latency, and error rates.
//!
//! ## Features
//!
//! - Metrics collection during pipeline execution
//! - Real-time metrics aggregation
//! - HTML visualization with embedded metrics and charts
//! - Metrics snapshot export

mod pipeline;

use pipeline::create_multi_stage_pipeline;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use streamweave::visualization::realtime::PipelineMetrics;
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

/// Enhances HTML with embedded metrics data and charts.
///
/// This function takes the standard HTML output and adds metrics visualization
/// including charts for throughput, latency, and error rates.
fn enhance_html_with_metrics(
  html: &str,
  metrics_snapshots: &[streamweave::visualization::realtime::NodeMetricsSnapshot],
) -> String {
  // Convert metrics to JSON
  let metrics_json = serde_json::to_string(metrics_snapshots).unwrap_or_else(|_| "[]".to_string());

  // Find the closing </body> tag and insert metrics visualization before it
  if let Some(body_pos) = html.rfind("</body>") {
    let mut enhanced = html.to_string();

    // Insert metrics data and visualization script
    let metrics_html = format!(
      r#"
    <script>
      // Embedded metrics data
      const metricsData = {};
      
      // Metrics visualization
      function renderMetrics() {{
        const metricsContainer = document.getElementById('metrics-container');
        if (!metricsContainer || !metricsData || metricsData.length === 0) {{
          return;
        }}
        
        let html = '<div style="margin: 20px; padding: 20px; background: #f5f5f5; border-radius: 8px;">';
        html += '<h2 style="margin-top: 0;">Pipeline Metrics</h2>';
        
        metricsData.forEach(node => {{
          html += '<div style="margin-bottom: 20px; padding: 15px; background: white; border-radius: 4px;">';
          html += '<h3 style="margin-top: 0;">Node: ' + escapeHtml(node.node_id) + '</h3>';
          html += '<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px;">';
          html += '<div><strong>Throughput:</strong> ' + node.throughput.toFixed(2) + ' items/sec</div>';
          html += '<div><strong>Total Items:</strong> ' + node.total_items + '</div>';
          html += '<div><strong>Errors:</strong> ' + node.error_count + '</div>';
          html += '<div><strong>Avg Latency:</strong> ' + node.avg_latency_ms.toFixed(2) + ' ms</div>';
          html += '<div><strong>P50 Latency:</strong> ' + node.p50_latency_ms.toFixed(2) + ' ms</div>';
          html += '<div><strong>P95 Latency:</strong> ' + node.p95_latency_ms.toFixed(2) + ' ms</div>';
          html += '<div><strong>P99 Latency:</strong> ' + node.p99_latency_ms.toFixed(2) + ' ms</div>';
          html += '</div>';
          html += '</div>';
        }});
        
        html += '</div>';
        metricsContainer.innerHTML = html;
      }}
      
      function escapeHtml(text) {{
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
      }}
      
      // Render metrics when page loads
      if (document.readyState === 'loading') {{
        document.addEventListener('DOMContentLoaded', renderMetrics);
      }} else {{
        renderMetrics();
      }}
    </script>
    
    <div id="metrics-container"></div>
"#,
      metrics_json
    );

    enhanced.insert_str(body_pos, &metrics_html);
    enhanced
  } else {
    html.to_string()
  }
}

/// Simulates pipeline execution and collects metrics.
///
/// This function simulates processing items through the pipeline and
/// collects metrics for each node.
async fn simulate_pipeline_execution(
  metrics: &PipelineMetrics,
  num_items: u64,
) -> Vec<streamweave::visualization::realtime::NodeMetricsSnapshot> {
  // Get or create metrics for each node
  let mut producer_metrics = metrics.get_or_create_node("producer".to_string()).await;
  let mut transformer1_metrics = metrics.get_or_create_node("transformer1".to_string()).await;
  let mut transformer2_metrics = metrics.get_or_create_node("transformer2".to_string()).await;
  let mut consumer_metrics = metrics.get_or_create_node("consumer".to_string()).await;

  // Simulate processing items
  for i in 0..num_items {
    // Producer processes item
    producer_metrics.record_item_processed().await;
    producer_metrics
      .record_latency(Duration::from_millis(1))
      .await;

    // Small delay to simulate processing
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Transformer1 processes item
    transformer1_metrics.record_item_processed().await;
    transformer1_metrics
      .record_latency(Duration::from_millis(2))
      .await;

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Transformer2 processes item
    transformer2_metrics.record_item_processed().await;
    transformer2_metrics
      .record_latency(Duration::from_millis(3))
      .await;

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Consumer processes item
    consumer_metrics.record_item_processed().await;
    consumer_metrics
      .record_latency(Duration::from_millis(1))
      .await;

    // Simulate occasional errors
    if i % 20 == 0 {
      transformer1_metrics.record_error().await;
    }
  }

  // Update metrics in the collection
  {
    let mut nodes = metrics.nodes.write().await;
    nodes.insert("producer".to_string(), producer_metrics);
    nodes.insert("transformer1".to_string(), transformer1_metrics);
    nodes.insert("transformer2".to_string(), transformer2_metrics);
    nodes.insert("consumer".to_string(), consumer_metrics);
  }

  // Get final snapshots
  metrics.snapshot_all().await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎨 StreamWeave Real-Time Metrics Visualization Example");
  println!("======================================================");
  println!();
  println!("This example demonstrates:");
  println!("1. Creating a multi-stage pipeline");
  println!("2. Collecting real-time metrics during execution");
  println!("3. Generating HTML visualization with embedded metrics");
  println!("4. Exporting metrics snapshots");
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

  // Initialize metrics collection
  println!("📈 Initializing metrics collection...");
  let pipeline_metrics = PipelineMetrics::new();
  println!("✅ Metrics collection initialized!");
  println!();

  // Simulate pipeline execution with metrics collection
  println!("🔄 Simulating pipeline execution (collecting metrics)...");
  let num_items = 50;
  let metrics_snapshots = simulate_pipeline_execution(&pipeline_metrics, num_items).await;
  println!("✅ Pipeline execution simulated!");
  println!("   Items processed: {}", num_items);
  println!("   Metrics collected for {} nodes", metrics_snapshots.len());
  println!();

  // Display metrics summary
  println!("📊 Metrics Summary:");
  for snapshot in &metrics_snapshots {
    println!("   {}:", snapshot.node_id);
    println!("     Throughput: {:.2} items/sec", snapshot.throughput);
    println!("     Total Items: {}", snapshot.total_items);
    println!("     Errors: {}", snapshot.error_count);
    println!("     Avg Latency: {:.2} ms", snapshot.avg_latency_ms);
    println!("     P95 Latency: {:.2} ms", snapshot.p95_latency_ms);
  }
  println!();

  // Generate base HTML
  println!("🌐 Generating HTML visualization...");
  let base_html = generate_standalone_html(&dag);

  // Enhance HTML with metrics
  let enhanced_html = enhance_html_with_metrics(&base_html, &metrics_snapshots);

  // Write enhanced HTML to file
  let output_path = PathBuf::from("pipeline_metrics_visualization.html");
  let mut file = File::create(&output_path)?;
  file.write_all(enhanced_html.as_bytes())?;

  println!("✅ Enhanced HTML file generated: {}", output_path.display());
  println!();

  // Export metrics as JSON
  println!("📝 Exporting metrics data...");
  let metrics_json = serde_json::to_string_pretty(&metrics_snapshots)?;
  std::fs::write("pipeline_metrics.json", &metrics_json)?;
  println!("✅ Metrics exported to: pipeline_metrics.json");
  println!();

  // Also export DAG formats
  println!("📝 Exporting additional formats...");
  let dag_json = dag.to_json()?;
  std::fs::write("pipeline_metrics_dag.json", dag_json)?;
  println!("   ✓ DAG JSON: pipeline_metrics_dag.json");

  let dag_dot = dag.to_dot();
  std::fs::write("pipeline_metrics_dag.dot", dag_dot)?;
  println!("   ✓ DAG DOT: pipeline_metrics_dag.dot");
  println!();

  println!("✅ Metrics visualization example completed successfully!");
  println!();
  println!("📋 Generated files:");
  println!(
    "   • {} - Interactive visualization with metrics",
    output_path.display()
  );
  println!("   • pipeline_metrics.json - Metrics data in JSON format");
  println!("   • pipeline_metrics_dag.json - DAG structure in JSON");
  println!("   • pipeline_metrics_dag.dot - DAG structure in DOT format");
  println!();
  println!("💡 Features:");
  println!("   • Real-time metrics collection during pipeline execution");
  println!("   • Throughput tracking (items per second)");
  println!("   • Latency percentiles (P50, P95, P99)");
  println!("   • Error rate monitoring");
  println!("   • Interactive HTML visualization with embedded metrics");
  println!();
  println!("💡 Instructions:");
  println!("   • Open the HTML file in your browser to view the visualization");
  println!("   • Metrics are displayed below the DAG visualization");
  println!("   • Use the JSON file for programmatic analysis");

  Ok(())
}
