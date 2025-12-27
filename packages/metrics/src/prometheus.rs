//! # Prometheus Metrics Export
//!
//! This module provides Prometheus metrics export for StreamWeave pipelines,
//! enabling integration with Prometheus monitoring systems.
//!
//! ## Features
//!
//! - **Metrics Export**: Export metrics in Prometheus text format
//! - **Standard Metrics**: Throughput, latency, errors, and backpressure
//! - **Label Support**: Configurable labels for metric identification
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::metrics::{MetricsCollector, PrometheusExporter};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let collector = MetricsCollector::new("my-pipeline");
//! let metrics_handle = collector.metrics();
//!
//! // Export metrics in Prometheus format
//! let exporter = PrometheusExporter::new("streamweave");
//! let prometheus_text = exporter.export(&metrics_handle.metrics());
//!
//! // Serve via HTTP endpoint (when HTTP server is available)
//! // GET /metrics -> returns prometheus_text
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "prometheus")]
use crate::types::PipelineMetrics;
#[cfg(feature = "prometheus")]
use std::collections::HashMap;

/// Exports StreamWeave metrics in Prometheus text format.
///
/// This exporter converts internal StreamWeave metrics to Prometheus-compatible
/// format that can be scraped by Prometheus servers.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::metrics::{MetricsCollector, PrometheusExporter};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let collector = MetricsCollector::new("my-pipeline");
/// let metrics_handle = collector.metrics();
///
/// let exporter = PrometheusExporter::new("streamweave");
/// let prometheus_text = exporter.export(&metrics_handle.metrics());
/// println!("{}", prometheus_text);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "prometheus")]
#[derive(Debug, Clone)]
pub struct PrometheusExporter {
  /// Metric name prefix for all exported metrics.
  metric_prefix: String,
  /// Default labels to include with all metrics.
  default_labels: HashMap<String, String>,
}

#[cfg(feature = "prometheus")]
impl PrometheusExporter {
  /// Creates a new Prometheus exporter.
  ///
  /// # Arguments
  ///
  /// * `metric_prefix` - Prefix for all metric names (e.g., "streamweave").
  ///
  /// # Returns
  ///
  /// A new `PrometheusExporter` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::metrics::PrometheusExporter;
  ///
  /// let exporter = PrometheusExporter::new("streamweave");
  /// ```
  #[must_use]
  pub fn new(metric_prefix: impl Into<String>) -> Self {
    Self {
      metric_prefix: metric_prefix.into(),
      default_labels: HashMap::new(),
    }
  }

  /// Adds a default label that will be included with all metrics.
  ///
  /// # Arguments
  ///
  /// * `key` - The label key.
  /// * `value` - The label value.
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::metrics::PrometheusExporter;
  ///
  /// let exporter = PrometheusExporter::new("streamweave")
  ///     .with_label("environment", "production")
  ///     .with_label("service", "api");
  /// ```
  #[must_use]
  pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.default_labels.insert(key.into(), value.into());
    self
  }

  /// Exports metrics in Prometheus text format.
  ///
  /// # Arguments
  ///
  /// * `metrics` - The pipeline metrics to export.
  ///
  /// # Returns
  ///
  /// A string containing metrics in Prometheus text format.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::metrics::{MetricsCollector, PrometheusExporter};
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let collector = MetricsCollector::new("my-pipeline");
  /// let metrics_handle = collector.metrics();
  ///
  /// let exporter = PrometheusExporter::new("streamweave");
  /// let prometheus_text = exporter.export(&metrics_handle.metrics());
  /// println!("{}", prometheus_text);
  /// # Ok(())
  /// # }
  /// ```
  pub fn export(&self, metrics: &PipelineMetrics) -> String {
    let mut output = String::new();

    // Format labels string
    let labels_str = self.format_labels(&self.default_labels);

    // Export throughput metrics
    let throughput = metrics.throughput();
    output.push_str(&format!(
      "# HELP {}_items_processed_total Total number of items processed\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_items_processed_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "items_processed",
      labels_str,
      throughput.items_processed()
    ));

    output.push_str(&format!(
      "# HELP {}_items_produced_total Total number of items produced\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_items_produced_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "items_produced",
      labels_str,
      throughput.items_produced()
    ));

    output.push_str(&format!(
      "# HELP {}_items_consumed_total Total number of items consumed\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_items_consumed_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "items_consumed",
      labels_str,
      throughput.items_consumed()
    ));

    output.push_str(&format!(
      "# HELP {}_items_per_second Items processed per second\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_items_per_second gauge\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}{}{:.2}\n",
      self.metric_prefix,
      "items_per_second",
      labels_str,
      throughput.items_per_second()
    ));

    // Export latency metrics
    let latency = metrics.latency();
    if let Some(avg) = latency.latency_avg() {
      output.push_str(&format!(
        "# HELP {}_latency_seconds_avg Average processing latency in seconds\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_latency_seconds_avg gauge\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_avg{}{:.9}\n",
        self.metric_prefix,
        "latency_seconds",
        labels_str,
        avg.as_secs_f64()
      ));
    }

    if let Some(min) = latency.latency_min() {
      output.push_str(&format!(
        "# HELP {}_latency_seconds_min Minimum processing latency in seconds\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_latency_seconds_min gauge\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_min{}{:.9}\n",
        self.metric_prefix,
        "latency_seconds",
        labels_str,
        min.as_secs_f64()
      ));
    }

    if let Some(max) = latency.latency_max() {
      output.push_str(&format!(
        "# HELP {}_latency_seconds_max Maximum processing latency in seconds\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_latency_seconds_max gauge\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_max{}{:.9}\n",
        self.metric_prefix,
        "latency_seconds",
        labels_str,
        max.as_secs_f64()
      ));
    }

    if let Some(p50) = latency.latency_p50() {
      output.push_str(&format!(
        "# HELP {}_latency_seconds_p50 50th percentile latency in seconds\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_latency_seconds_p50 gauge\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_p50{}{:.9}\n",
        self.metric_prefix,
        "latency_seconds",
        labels_str,
        p50.as_secs_f64()
      ));
    }

    if let Some(p95) = latency.latency_p95() {
      output.push_str(&format!(
        "# HELP {}_latency_seconds_p95 95th percentile latency in seconds\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_latency_seconds_p95 gauge\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_p95{}{:.9}\n",
        self.metric_prefix,
        "latency_seconds",
        labels_str,
        p95.as_secs_f64()
      ));
    }

    if let Some(p99) = latency.latency_p99() {
      output.push_str(&format!(
        "# HELP {}_latency_seconds_p99 99th percentile latency in seconds\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_latency_seconds_p99 gauge\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_p99{}{:.9}\n",
        self.metric_prefix,
        "latency_seconds",
        labels_str,
        p99.as_secs_f64()
      ));
    }

    // Export error metrics
    let errors = metrics.errors();
    output.push_str(&format!(
      "# HELP {}_errors_total Total number of errors\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_errors_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "errors",
      labels_str,
      errors.total_errors()
    ));

    output.push_str(&format!(
      "# HELP {}_errors_fatal_total Total number of fatal errors\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_errors_fatal_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "errors_fatal",
      labels_str,
      errors.fatal_errors()
    ));

    output.push_str(&format!(
      "# HELP {}_errors_skipped_total Total number of skipped errors\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_errors_skipped_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "errors_skipped",
      labels_str,
      errors.skipped_errors()
    ));

    output.push_str(&format!(
      "# HELP {}_errors_retried_total Total number of retried errors\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_errors_retried_total counter\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}_total{}{}\n",
      self.metric_prefix,
      "errors_retried",
      labels_str,
      errors.retried_errors()
    ));

    // Export errors by type
    for (error_type, count) in errors.errors_by_type() {
      let mut error_labels = self.default_labels.clone();
      error_labels.insert("error_type".to_string(), error_type.clone());
      let error_labels_str = self.format_labels(&error_labels);

      output.push_str(&format!(
        "# HELP {}_errors_by_type_total Total number of errors by type\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "# TYPE {}_errors_by_type_total counter\n",
        self.metric_prefix
      ));
      output.push_str(&format!(
        "{}_{}_total{}{}\n",
        self.metric_prefix, "errors_by_type", error_labels_str, count
      ));
    }

    // Export backpressure level
    let backpressure = metrics.backpressure();
    output.push_str(&format!(
      "# HELP {}_backpressure_level Current backpressure level (0=none, 1=low, 2=medium, 3=high)\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "# TYPE {}_backpressure_level gauge\n",
      self.metric_prefix
    ));
    output.push_str(&format!(
      "{}_{}{}{}\n",
      self.metric_prefix, "backpressure_level", labels_str, backpressure as u8
    ));

    output
  }

  /// Formats labels as a Prometheus label string.
  ///
  /// # Arguments
  ///
  /// * `labels` - Map of label key-value pairs.
  ///
  /// # Returns
  ///
  /// Formatted label string (e.g., `{key1="value1",key2="value2"}`).
  fn format_labels(&self, labels: &HashMap<String, String>) -> String {
    if labels.is_empty() {
      return String::new();
    }

    let mut label_parts: Vec<String> = labels
      .iter()
      .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('"', "\\\"")))
      .collect();
    label_parts.sort();

    format!("{{{}}}", label_parts.join(","))
  }
}

#[cfg(not(feature = "prometheus"))]
// Placeholder implementation for when Prometheus is not enabled
pub struct PrometheusExporter;

#[cfg(not(feature = "prometheus"))]
impl PrometheusExporter {
  #[must_use]
  pub fn new(_metric_prefix: impl Into<String>) -> Self {
    Self
  }

  #[must_use]
  pub fn with_label(self, _key: impl Into<String>, _value: impl Into<String>) -> Self {
    self
  }

  pub fn export(&self, _metrics: &crate::types::PipelineMetrics) -> String {
    String::new()
  }
}

#[cfg(test)]
#[cfg(feature = "prometheus")]
mod tests {
  use super::*;
  use crate::types::PipelineMetrics;

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

  #[test]
  fn test_format_labels() {
    let exporter = PrometheusExporter::new("test");
    let mut labels = HashMap::new();
    labels.insert("key1".to_string(), "value1".to_string());
    labels.insert("key2".to_string(), "value2".to_string());

    let formatted = exporter.format_labels(&labels);
    assert!(formatted.contains("key1=\"value1\""));
    assert!(formatted.contains("key2=\"value2\""));
    assert!(formatted.starts_with('{'));
    assert!(formatted.ends_with('}'));
  }
}
