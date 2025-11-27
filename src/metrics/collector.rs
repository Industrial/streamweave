//! # Metrics Collector
//!
//! The metrics collector is responsible for tracking metrics during pipeline execution.
//! It provides a handle that can be used to access metrics at any time.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::metrics::MetricsCollector;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let collector = MetricsCollector::new("my-pipeline");
//! let metrics_handle = collector.metrics();
//!
//! // Use metrics_handle to access metrics during/after pipeline execution
//! let throughput = metrics_handle.throughput().items_per_second();
//! # Ok(())
//! # }
//! ```

use crate::metrics::types::PipelineMetrics;
use std::sync::Arc;

/// Metrics collector for pipeline observability.
///
/// The collector tracks metrics during pipeline execution and provides access
/// to those metrics through a shared handle.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::metrics::MetricsCollector;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let collector = MetricsCollector::new("my-pipeline");
/// let metrics = collector.metrics();
///
/// // Metrics are automatically collected during pipeline execution
/// // Access current metrics:
/// let throughput = metrics.throughput().items_per_second();
/// let latency = metrics.latency().latency_p95();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MetricsCollector {
  /// The pipeline metrics being collected.
  metrics: Arc<PipelineMetrics>,
}

impl MetricsCollector {
  /// Creates a new metrics collector.
  ///
  /// # Arguments
  ///
  /// * `pipeline_name` - Name/identifier for the pipeline being monitored.
  ///
  /// # Returns
  ///
  /// A new `MetricsCollector` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::MetricsCollector;
  ///
  /// let collector = MetricsCollector::new("data-processing-pipeline");
  /// ```
  #[must_use]
  pub fn new(pipeline_name: impl Into<String>) -> Self {
    Self {
      metrics: Arc::new(PipelineMetrics::new(pipeline_name)),
    }
  }

  /// Gets a handle to the metrics being collected.
  ///
  /// The handle can be used to access metrics at any time, even during
  /// pipeline execution.
  ///
  /// # Returns
  ///
  /// A `MetricsHandle` providing access to the collected metrics.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::MetricsCollector;
  ///
  /// let collector = MetricsCollector::new("my-pipeline");
  /// let metrics = collector.metrics();
  ///
  /// // Access metrics
  /// let throughput = metrics.throughput().items_processed();
  /// ```
  #[must_use]
  pub fn metrics(&self) -> MetricsHandle {
    MetricsHandle {
      metrics: Arc::clone(&self.metrics),
    }
  }

  /// Gets the underlying pipeline metrics.
  ///
  /// This method is useful for direct access to metrics without the handle wrapper.
  ///
  /// # Returns
  ///
  /// A reference to the `PipelineMetrics` instance.
  #[must_use]
  pub fn pipeline_metrics(&self) -> &PipelineMetrics {
    &self.metrics
  }
}

impl Default for MetricsCollector {
  fn default() -> Self {
    Self::new("unnamed-pipeline")
  }
}

/// Handle providing access to collected metrics.
///
/// This handle can be cloned and shared across threads, allowing multiple
/// parts of the codebase to access metrics concurrently.
///
/// # Example
///
/// ```rust
/// use streamweave::metrics::{MetricsCollector, MetricsHandle};
///
/// let collector = MetricsCollector::new("my-pipeline");
/// let handle1 = collector.metrics();
/// let handle2 = collector.metrics(); // Clone of same handle
///
/// // Both handles access the same underlying metrics
/// assert_eq!(
///     handle1.throughput().items_processed(),
///     handle2.throughput().items_processed()
/// );
/// ```
#[derive(Debug, Clone)]
pub struct MetricsHandle {
  /// Shared reference to the pipeline metrics.
  metrics: Arc<PipelineMetrics>,
}

impl MetricsHandle {
  /// Gets a reference to the pipeline metrics.
  ///
  /// # Returns
  ///
  /// A reference to the `PipelineMetrics` instance.
  #[must_use]
  pub fn metrics(&self) -> &PipelineMetrics {
    &self.metrics
  }

  /// Gets throughput metrics.
  ///
  /// # Returns
  ///
  /// A reference to the throughput metrics.
  #[must_use]
  pub fn throughput(&self) -> &crate::metrics::types::ThroughputMetrics {
    self.metrics.throughput()
  }

  /// Gets latency metrics.
  ///
  /// # Returns
  ///
  /// A reference to the latency metrics.
  #[must_use]
  pub fn latency(&self) -> &crate::metrics::types::LatencyMetrics {
    self.metrics.latency()
  }

  /// Gets error metrics.
  ///
  /// # Returns
  ///
  /// A reference to the error metrics.
  #[must_use]
  pub fn errors(&self) -> &crate::metrics::types::ErrorMetrics {
    self.metrics.errors()
  }

  /// Gets the pipeline name.
  ///
  /// # Returns
  ///
  /// The pipeline identifier.
  #[must_use]
  pub fn pipeline_name(&self) -> &str {
    self.metrics.name()
  }

  /// Gets the elapsed time since metrics collection started.
  ///
  /// # Returns
  ///
  /// The elapsed duration.
  #[must_use]
  pub fn elapsed(&self) -> std::time::Duration {
    self.metrics.elapsed()
  }

  /// Gets the current backpressure level.
  ///
  /// # Returns
  ///
  /// The current backpressure level.
  #[must_use]
  pub fn backpressure(&self) -> crate::metrics::types::BackpressureLevel {
    self.metrics.backpressure()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_metrics_collector() {
    let collector = MetricsCollector::new("test-pipeline");
    let handle1 = collector.metrics();
    let handle2 = collector.metrics();

    assert_eq!(handle1.pipeline_name(), "test-pipeline");
    assert_eq!(handle1.pipeline_name(), handle2.pipeline_name());

    // Both handles should access the same metrics
    handle1.metrics().throughput().increment_items_processed(10);
    assert_eq!(handle2.metrics().throughput().items_processed(), 10);
  }

  #[test]
  fn test_metrics_handle() {
    let collector = MetricsCollector::new("handle-test");
    let handle = collector.metrics();

    assert_eq!(handle.pipeline_name(), "handle-test");
    assert_eq!(handle.throughput().items_processed(), 0);
    assert_eq!(handle.errors().total_errors(), 0);
  }
}
