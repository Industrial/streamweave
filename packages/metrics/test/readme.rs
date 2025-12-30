//! Integration tests for README examples.
//!
//! These tests verify that the code examples in the README.md file compile and work correctly.

use std::time::Duration;
use streamweave_metrics::{
  MetricsCollector, PipelineMetrics,
  health::{ComponentHealth, HealthCheck, HealthStatus},
};

#[test]
fn test_basic_metrics_collection() {
  // Example from README: Basic Metrics Collection
  let collector = MetricsCollector::new("my-pipeline");
  let metrics_handle = collector.metrics();

  // Metrics are automatically collected during pipeline execution
  let throughput = metrics_handle.metrics().throughput();
  let latency = metrics_handle.metrics().latency();

  // Verify we can access the metrics
  assert_eq!(throughput.items_processed(), 0);
  assert!(latency.latency_avg().is_none());
}

#[test]
fn test_health_checks() {
  // Example from README: Health Checks
  let health_check = HealthCheck::new("my-pipeline");

  health_check.set_component_health("database", ComponentHealth::healthy());
  health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));

  let status = health_check.status();
  let is_ready = health_check.is_ready();

  // Verify the health check works
  assert!(!status.is_healthy()); // Should be degraded due to cache
  assert!(is_ready); // But still ready
  assert!(status.reason().is_some());
}

#[test]
#[cfg(feature = "prometheus")]
fn test_prometheus_export() {
  // Example from README: Prometheus Export
  use streamweave_metrics::PrometheusExporter;

  let collector = MetricsCollector::new("my-pipeline");
  let metrics_handle = collector.metrics();

  let exporter = PrometheusExporter::new("streamweave");
  let prometheus_text = exporter.export(&metrics_handle.metrics());

  // Verify we get some output
  assert!(!prometheus_text.is_empty());
  assert!(prometheus_text.contains("streamweave"));
}

#[test]
fn test_throughput_monitoring() {
  // Example from README: Throughput Monitoring
  let collector = MetricsCollector::new("my-pipeline");
  let metrics_handle = collector.metrics();

  // Access throughput metrics
  let throughput = metrics_handle.metrics().throughput();
  let items_processed = throughput.items_processed();

  assert_eq!(items_processed, 0);
}

#[test]
fn test_latency_tracking() {
  // Example from README: Latency Tracking (adapted - record_latency is on PipelineMetrics)
  let collector = MetricsCollector::new("my-pipeline");
  let metrics_handle = collector.metrics();

  // Record latency through the metrics
  metrics_handle
    .metrics()
    .record_latency(Duration::from_millis(100));

  // Access latency metrics
  let latency = metrics_handle.metrics().latency();
  let avg = latency.latency_avg();

  assert!(avg.is_some());
}

#[test]
fn test_health_status() {
  // Example from README: Health Status
  let health_check = HealthCheck::new("my-pipeline");

  // Set component health
  health_check.set_component_health("database", ComponentHealth::healthy());

  health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));

  // Check overall health
  let status = health_check.status();
  match status {
    HealthStatus::Healthy => {
      panic!("Should not be healthy due to degraded cache");
    }
    HealthStatus::Degraded { reason } => {
      assert!(reason.contains("cache") || reason.contains("latency"));
    }
    HealthStatus::Unhealthy { reason: _ } => {
      panic!("Should not be unhealthy");
    }
  }
}
