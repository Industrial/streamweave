#[cfg(feature = "prometheus")]
use std::time::Duration;
#[cfg(feature = "prometheus")]
use streamweave_metrics::{PrometheusExporter, types::PipelineMetrics};

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export() {
  let metrics = PipelineMetrics::new("test-pipeline");
  let exporter = PrometheusExporter::new("streamweave");

  let output = exporter.export(&metrics);

  // Verify output contains expected metric names
  assert!(output.contains("streamweave_items_processed_total"));
  assert!(output.contains("streamweave_items_per_second"));
  assert!(output.contains("streamweave_errors_total"));
  assert!(output.contains("streamweave_backpressure_level"));
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_with_labels() {
  let metrics = PipelineMetrics::new("test-pipeline");
  let exporter = PrometheusExporter::new("streamweave")
    .with_label("environment", "test")
    .with_label("service", "api");

  let output = exporter.export(&metrics);

  // Verify labels are included
  assert!(output.contains("environment=\"test\""));
  assert!(output.contains("service=\"api\""));
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_with_data() {
  let metrics = PipelineMetrics::new("test-pipeline");
  metrics.throughput().increment_items_processed(100);
  metrics.throughput().increment_items_produced(80);
  metrics.throughput().increment_items_consumed(90);
  metrics.record_latency(Duration::from_millis(50));
  metrics.errors().record_error("TestError");
  metrics.errors().record_fatal_error();
  metrics.errors().record_skipped_error();
  metrics.errors().record_retried_error();
  metrics.set_backpressure(crate::types::BackpressureLevel::High);

  let exporter = PrometheusExporter::new("streamweave");
  let output = exporter.export(&metrics);

  // Verify all metrics are present
  assert!(output.contains("streamweave_items_processed_total"));
  assert!(output.contains("streamweave_items_produced_total"));
  assert!(output.contains("streamweave_items_consumed_total"));
  assert!(output.contains("streamweave_items_per_second"));
  assert!(output.contains("streamweave_errors_total"));
  assert!(output.contains("streamweave_errors_fatal_total"));
  assert!(output.contains("streamweave_errors_skipped_total"));
  assert!(output.contains("streamweave_errors_retried_total"));
  assert!(output.contains("streamweave_backpressure_level"));

  // Verify values
  assert!(output.contains("100")); // items_processed
  assert!(output.contains("80")); // items_produced
  assert!(output.contains("90")); // items_consumed
  assert!(output.contains("1")); // errors_total
  assert!(output.contains("3")); // backpressure_level (High)
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_latency_metrics() {
  let metrics = PipelineMetrics::new("test-pipeline");
  metrics.record_latency(Duration::from_millis(10));
  metrics.record_latency(Duration::from_millis(20));
  metrics.record_latency(Duration::from_millis(30));

  let exporter = PrometheusExporter::new("streamweave");
  let output = exporter.export(&metrics);

  // Verify latency metrics are present
  assert!(output.contains("streamweave_latency_seconds_avg"));
  assert!(output.contains("streamweave_latency_seconds_min"));
  assert!(output.contains("streamweave_latency_seconds_max"));
  assert!(output.contains("streamweave_latency_seconds_p50"));
  assert!(output.contains("streamweave_latency_seconds_p95"));
  assert!(output.contains("streamweave_latency_seconds_p99"));
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_errors_by_type() {
  let metrics = PipelineMetrics::new("test-pipeline");
  metrics.errors().record_error("ParseError");
  metrics.errors().record_error("NetworkError");
  metrics.errors().record_error("ParseError");

  let exporter = PrometheusExporter::new("streamweave");
  let output = exporter.export(&metrics);

  // Verify errors by type are exported
  assert!(output.contains("streamweave_errors_by_type_total"));
  assert!(output.contains("error_type=\"ParseError\""));
  assert!(output.contains("error_type=\"NetworkError\""));
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_empty_labels() {
  let metrics = PipelineMetrics::new("test-pipeline");
  let exporter = PrometheusExporter::new("streamweave");
  let output = exporter.export(&metrics);

  // With no labels, metrics should not have label brackets
  // Check that we don't have empty {} labels
  let lines: Vec<&str> = output.lines().collect();
  for line in lines {
    if line.starts_with("streamweave_") && !line.starts_with("#") {
      // Should not have empty {} labels
      assert!(!line.contains("{}"));
    }
  }
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_label_escaping() {
  let metrics = PipelineMetrics::new("test-pipeline");
  let exporter = PrometheusExporter::new("streamweave").with_label("key", "value with \"quotes\"");

  let output = exporter.export(&metrics);

  // Verify quotes are escaped
  assert!(output.contains("value with \\\"quotes\\\""));
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_export_label_sorting() {
  let metrics = PipelineMetrics::new("test-pipeline");
  let exporter = PrometheusExporter::new("streamweave")
    .with_label("zebra", "last")
    .with_label("alpha", "first");

  let output = exporter.export(&metrics);

  // Labels should be sorted alphabetically
  // Check that alpha comes before zebra in the label string
  let lines: Vec<&str> = output.lines().collect();
  for line in lines {
    if line.contains("alpha") && line.contains("zebra") {
      let alpha_pos = line.find("alpha").unwrap();
      let zebra_pos = line.find("zebra").unwrap();
      assert!(alpha_pos < zebra_pos);
    }
  }
}

#[cfg(feature = "prometheus")]
#[test]
fn test_prometheus_exporter_clone() {
  let exporter1 = PrometheusExporter::new("test").with_label("env", "prod");
  let exporter2 = exporter1.clone();

  let metrics = PipelineMetrics::new("test");
  let output1 = exporter1.export(&metrics);
  let output2 = exporter2.export(&metrics);

  // Both should produce same output
  assert_eq!(output1, output2);
}

#[cfg(not(feature = "prometheus"))]
#[test]
fn test_prometheus_exporter_placeholder() {
  use streamweave_metrics::PipelineMetrics;
  use streamweave_metrics::PrometheusExporter;

  let exporter = PrometheusExporter::new("test");
  let metrics = PipelineMetrics::new("test");
  let output = exporter.export(&metrics);

  // Placeholder should return empty string
  assert!(output.is_empty());
}

// Note: test_format_labels was removed because format_labels is a private method
// and cannot be accessed from integration tests. The functionality is indirectly
// tested through test_prometheus_export_with_labels.
