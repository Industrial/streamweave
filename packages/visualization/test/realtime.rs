use std::time::Duration;
use streamweave_visualization::{NodeMetrics, PipelineMetrics};

#[tokio::test]
async fn test_node_metrics_creation() {
  let metrics = NodeMetrics::new("test_node".to_string());
  assert_eq!(metrics.node_id, "test_node");
  assert_eq!(metrics.throughput, 0.0);
  assert_eq!(metrics.total_items, 0);
}

#[tokio::test]
async fn test_node_metrics_record_item() {
  let mut metrics = NodeMetrics::new("test_node".to_string());
  metrics.record_item_processed().await;
  assert_eq!(metrics.total_items, 1);
}

#[tokio::test]
async fn test_node_metrics_record_latency() {
  let mut metrics = NodeMetrics::new("test_node".to_string());
  metrics.record_latency(Duration::from_millis(10)).await;
  assert!(metrics.avg_latency_ms > 0.0);
}

#[tokio::test]
async fn test_pipeline_metrics_bottleneck_detection() {
  let metrics = PipelineMetrics::new();

  // Create metrics for two nodes
  let mut node1 = metrics.get_or_create_node("node1".to_string()).await;
  node1.throughput = 100.0;
  {
    let mut nodes = metrics.nodes.write().await;
    nodes.insert("node1".to_string(), node1);
  }

  let mut node2 = metrics.get_or_create_node("node2".to_string()).await;
  node2.throughput = 50.0; // 50% reduction - bottleneck
  {
    let mut nodes = metrics.nodes.write().await;
    nodes.insert("node2".to_string(), node2);
  }

  let edges = vec![streamweave_visualization::DagEdge::new(
    "node1".to_string(),
    "node2".to_string(),
    None,
  )];

  let bottlenecks = metrics.detect_bottlenecks(&edges).await;
  assert!(bottlenecks.contains(&"node2".to_string()));
}
