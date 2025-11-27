//! # Real-Time Visualization Metrics
//!
//! This module provides types and utilities for tracking and exposing real-time
//! metrics for pipeline visualization, including throughput, latency, and bottleneck detection.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::visualization::realtime::NodeMetrics;
//!
//! let mut metrics = NodeMetrics::new("producer".to_string());
//! metrics.record_item_processed();
//! let throughput = metrics.current_throughput();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

fn default_instant() -> Arc<RwLock<Instant>> {
  Arc::new(RwLock::new(Instant::now()))
}

/// Metrics for a single pipeline node in real-time visualization.
///
/// Tracks throughput, latency, and other metrics for a specific node
/// to enable real-time visualization of data flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
  /// Node identifier
  pub node_id: String,
  /// Current throughput (items per second)
  pub throughput: f64,
  /// Average latency in milliseconds
  pub avg_latency_ms: f64,
  /// P50 latency in milliseconds
  pub p50_latency_ms: f64,
  /// P95 latency in milliseconds
  pub p95_latency_ms: f64,
  /// P99 latency in milliseconds
  pub p99_latency_ms: f64,
  /// Total items processed
  pub total_items: u64,
  /// Error count
  pub error_count: u64,
  /// Last update timestamp
  #[serde(skip, default = "default_instant")]
  pub last_update: Arc<RwLock<Instant>>,
  /// Items processed in the last time window
  #[serde(skip)]
  items_window: Arc<RwLock<VecDeque<Instant>>>,
  /// Latency samples for percentile calculation
  #[serde(skip)]
  latency_samples: Arc<RwLock<VecDeque<Duration>>>,
}

impl NodeMetrics {
  /// Creates new node metrics for the given node ID.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The identifier of the node these metrics are for
  ///
  /// # Returns
  ///
  /// A new `NodeMetrics` instance initialized to zero.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::NodeMetrics;
  ///
  /// let metrics = NodeMetrics::new("producer_1".to_string());
  /// ```
  #[must_use]
  pub fn new(node_id: String) -> Self {
    Self {
      node_id,
      throughput: 0.0,
      avg_latency_ms: 0.0,
      p50_latency_ms: 0.0,
      p95_latency_ms: 0.0,
      p99_latency_ms: 0.0,
      total_items: 0,
      error_count: 0,
      last_update: Arc::new(RwLock::new(Instant::now())),
      items_window: Arc::new(RwLock::new(VecDeque::new())),
      latency_samples: Arc::new(RwLock::new(VecDeque::new())),
    }
  }

  /// Records that an item was processed by this node.
  ///
  /// Updates throughput calculation and total item count.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::NodeMetrics;
  ///
  /// # async fn example() {
  /// let mut metrics = NodeMetrics::new("transformer_1".to_string());
  /// metrics.record_item_processed().await;
  /// # }
  /// ```
  pub async fn record_item_processed(&mut self) {
    let now = Instant::now();
    {
      let mut window = self.items_window.write().await;
      window.push_back(now);

      // Keep only last 60 seconds of items
      let cutoff = now.checked_sub(Duration::from_secs(60)).unwrap_or(now);
      while window.front().is_some_and(|&t| t < cutoff) {
        window.pop_front();
      }
    }

    {
      let mut last_update = self.last_update.write().await;
      *last_update = now;
    }

    self.total_items += 1;
    self.update_throughput().await;
  }

  /// Records the latency for processing an item.
  ///
  /// # Arguments
  ///
  /// * `latency` - The time taken to process the item
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::NodeMetrics;
  /// use std::time::Duration;
  ///
  /// # async fn example() {
  /// let mut metrics = NodeMetrics::new("transformer_1".to_string());
  /// metrics.record_latency(Duration::from_millis(10)).await;
  /// # }
  /// ```
  pub async fn record_latency(&mut self, latency: Duration) {
    {
      let mut samples = self.latency_samples.write().await;
      samples.push_back(latency);

      // Keep only last 1000 samples
      if samples.len() > 1000 {
        samples.pop_front();
      }
    }

    self.update_latency_stats().await;
  }

  /// Records an error occurring at this node.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::NodeMetrics;
  ///
  /// # async fn example() {
  /// let mut metrics = NodeMetrics::new("transformer_1".to_string());
  /// metrics.record_error().await;
  /// # }
  /// ```
  pub async fn record_error(&mut self) {
    self.error_count += 1;
  }

  /// Gets the current throughput in items per second.
  ///
  /// # Returns
  ///
  /// The current throughput calculated over the last time window.
  #[must_use]
  pub fn current_throughput(&self) -> f64 {
    self.throughput
  }

  /// Gets the average latency in milliseconds.
  ///
  /// # Returns
  ///
  /// The average latency over recent samples.
  #[must_use]
  pub fn average_latency_ms(&self) -> f64 {
    self.avg_latency_ms
  }

  async fn update_throughput(&mut self) {
    let window = self.items_window.read().await;
    let now = Instant::now();
    let window_duration = Duration::from_secs(5); // 5 second window
    let cutoff = now.checked_sub(window_duration).unwrap_or(now);

    let items_in_window = window.iter().filter(|&&t| t >= cutoff).count();
    self.throughput = items_in_window as f64 / window_duration.as_secs_f64();
  }

  async fn update_latency_stats(&mut self) {
    let samples = self.latency_samples.read().await;
    if samples.is_empty() {
      return;
    }

    let mut sorted_latencies: Vec<Duration> = samples.iter().copied().collect();
    sorted_latencies.sort();

    // Calculate average
    let sum: Duration = sorted_latencies.iter().sum();
    self.avg_latency_ms = sum.as_secs_f64() * 1000.0 / sorted_latencies.len() as f64;

    // Calculate percentiles
    if !sorted_latencies.is_empty() {
      self.p50_latency_ms = percentile(&sorted_latencies, 0.50).as_secs_f64() * 1000.0;
      self.p95_latency_ms = percentile(&sorted_latencies, 0.95).as_secs_f64() * 1000.0;
      self.p99_latency_ms = percentile(&sorted_latencies, 0.99).as_secs_f64() * 1000.0;
    }
  }

  /// Creates a serializable snapshot of current metrics.
  ///
  /// This snapshot can be sent over the network for real-time visualization.
  ///
  /// # Returns
  ///
  /// A snapshot of current metrics that can be serialized.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::NodeMetrics;
  ///
  /// # async fn example() {
  /// let metrics = NodeMetrics::new("node_1".to_string());
  /// let snapshot = metrics.snapshot();
  /// let json = serde_json::to_string(&snapshot).unwrap();
  /// # }
  /// ```
  #[must_use]
  pub fn snapshot(&self) -> NodeMetricsSnapshot {
    NodeMetricsSnapshot {
      node_id: self.node_id.clone(),
      throughput: self.throughput,
      avg_latency_ms: self.avg_latency_ms,
      p50_latency_ms: self.p50_latency_ms,
      p95_latency_ms: self.p95_latency_ms,
      p99_latency_ms: self.p99_latency_ms,
      total_items: self.total_items,
      error_count: self.error_count,
    }
  }
}

impl Default for NodeMetrics {
  fn default() -> Self {
    Self::new(String::new())
  }
}

/// A serializable snapshot of node metrics for real-time visualization.
///
/// This is a lightweight snapshot that can be easily serialized and sent
/// over the network to update visualization displays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetricsSnapshot {
  /// Node identifier
  pub node_id: String,
  /// Current throughput (items per second)
  pub throughput: f64,
  /// Average latency in milliseconds
  pub avg_latency_ms: f64,
  /// P50 latency in milliseconds
  pub p50_latency_ms: f64,
  /// P95 latency in milliseconds
  pub p95_latency_ms: f64,
  /// P99 latency in milliseconds
  pub p99_latency_ms: f64,
  /// Total items processed
  pub total_items: u64,
  /// Error count
  pub error_count: u64,
}

/// Collection of metrics for all nodes in a pipeline.
///
/// Manages metrics for multiple nodes and provides utilities for
/// bottleneck detection and overall pipeline health.
#[derive(Debug, Clone)]
pub struct PipelineMetrics {
  /// Metrics for each node by node ID
  pub nodes: Arc<RwLock<std::collections::HashMap<String, NodeMetrics>>>,
}

impl PipelineMetrics {
  /// Creates a new pipeline metrics collection.
  ///
  /// # Returns
  ///
  /// A new empty `PipelineMetrics` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::PipelineMetrics;
  ///
  /// let metrics = PipelineMetrics::new();
  /// ```
  #[must_use]
  pub fn new() -> Self {
    Self {
      nodes: Arc::new(RwLock::new(std::collections::HashMap::new())),
    }
  }

  /// Gets or creates metrics for a node.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The identifier of the node
  ///
  /// # Returns
  ///
  /// A clone of the metrics for the node.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::PipelineMetrics;
  ///
  /// # async fn example() {
  /// let metrics = PipelineMetrics::new();
  /// let node_metrics = metrics.get_or_create_node("producer_1".to_string()).await;
  /// # }
  /// ```
  pub async fn get_or_create_node(&self, node_id: String) -> NodeMetrics {
    let mut nodes = self.nodes.write().await;
    nodes
      .entry(node_id.clone())
      .or_insert_with(|| NodeMetrics::new(node_id.clone()))
      .clone()
  }

  /// Gets a snapshot of all node metrics.
  ///
  /// # Returns
  ///
  /// A vector of metric snapshots for all nodes.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::PipelineMetrics;
  ///
  /// # async fn example() {
  /// let metrics = PipelineMetrics::new();
  /// let snapshots = metrics.snapshot_all().await;
  /// let json = serde_json::to_string(&snapshots).unwrap();
  /// # }
  /// ```
  pub async fn snapshot_all(&self) -> Vec<NodeMetricsSnapshot> {
    let nodes = self.nodes.read().await;
    nodes.values().map(|m| m.snapshot()).collect()
  }

  /// Detects bottlenecks in the pipeline based on throughput.
  ///
  /// A bottleneck is identified as a node with significantly lower throughput
  /// than its predecessor, indicating it's slowing down the pipeline.
  ///
  /// # Arguments
  ///
  /// * `dag_edges` - Vector of edges representing data flow
  ///
  /// # Returns
  ///
  /// A vector of node IDs that are identified as bottlenecks.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::realtime::PipelineMetrics;
  /// use streamweave::visualization::DagEdge;
  ///
  /// # async fn example() {
  /// let metrics = PipelineMetrics::new();
  /// let edges = vec![
  ///     DagEdge::new("producer".to_string(), "transformer".to_string(), None),
  /// ];
  /// let bottlenecks = metrics.detect_bottlenecks(&edges).await;
  /// # }
  /// ```
  pub async fn detect_bottlenecks(
    &self,
    dag_edges: &[crate::visualization::DagEdge],
  ) -> Vec<String> {
    let nodes = self.nodes.read().await;
    let mut bottlenecks = Vec::new();

    for edge in dag_edges {
      let from_throughput = nodes.get(&edge.from).map(|m| m.throughput).unwrap_or(0.0);
      let to_throughput = nodes.get(&edge.to).map(|m| m.throughput).unwrap_or(0.0);

      // If downstream throughput is significantly lower (more than 20% reduction),
      // the downstream node is a bottleneck
      if from_throughput > 0.0 && to_throughput < from_throughput * 0.8 {
        bottlenecks.push(edge.to.clone());
      }
    }

    bottlenecks
  }
}

impl Default for PipelineMetrics {
  fn default() -> Self {
    Self::new()
  }
}

/// Calculates a percentile value from a sorted slice.
///
/// # Arguments
///
/// * `sorted_data` - A sorted slice of durations
/// * `percentile` - The percentile to calculate (0.0 to 1.0)
///
/// # Returns
///
/// The value at the given percentile.
fn percentile(sorted_data: &[Duration], percentile: f64) -> Duration {
  if sorted_data.is_empty() {
    return Duration::ZERO;
  }

  if sorted_data.len() == 1 {
    return sorted_data[0];
  }

  let index = (sorted_data.len() as f64 * percentile).ceil() as usize - 1;
  sorted_data[index.min(sorted_data.len() - 1)]
}

#[cfg(test)]
mod tests {
  use super::*;

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

    let edges = vec![crate::visualization::DagEdge::new(
      "node1".to_string(),
      "node2".to_string(),
      None,
    )];

    let bottlenecks = metrics.detect_bottlenecks(&edges).await;
    assert!(bottlenecks.contains(&"node2".to_string()));
  }
}
