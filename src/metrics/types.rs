//! # Metrics Types
//!
//! Standard metric types for pipeline observability.
//!
//! This module defines the core metric types that track pipeline performance,
//! including throughput, latency, errors, and backpressure indicators.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics for tracking throughput (items per second).
///
/// Throughput metrics track the rate at which items are processed through the pipeline.
///
/// # Example
///
/// ```rust
/// use streamweave::metrics::ThroughputMetrics;
///
/// let metrics = ThroughputMetrics::new();
/// metrics.increment_items_processed(100);
/// let throughput = metrics.items_per_second();
/// ```
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
  /// Total number of items processed.
  items_processed: Arc<AtomicU64>,
  /// Number of items produced.
  items_produced: Arc<AtomicU64>,
  /// Number of items consumed.
  items_consumed: Arc<AtomicU64>,
  /// Start time for throughput calculation.
  start_time: Arc<Instant>,
}

impl ThroughputMetrics {
  /// Creates a new throughput metrics tracker.
  ///
  /// # Returns
  ///
  /// A new `ThroughputMetrics` instance initialized to zero.
  #[must_use]
  pub fn new() -> Self {
    Self {
      items_processed: Arc::new(AtomicU64::new(0)),
      items_produced: Arc::new(AtomicU64::new(0)),
      items_consumed: Arc::new(AtomicU64::new(0)),
      start_time: Arc::new(Instant::now()),
    }
  }

  /// Increments the count of items processed.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of items to add to the processed count.
  pub fn increment_items_processed(&self, count: u64) {
    self.items_processed.fetch_add(count, Ordering::Relaxed);
  }

  /// Increments the count of items produced.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of items to add to the produced count.
  pub fn increment_items_produced(&self, count: u64) {
    self.items_produced.fetch_add(count, Ordering::Relaxed);
  }

  /// Increments the count of items consumed.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of items to add to the consumed count.
  pub fn increment_items_consumed(&self, count: u64) {
    self.items_consumed.fetch_add(count, Ordering::Relaxed);
  }

  /// Gets the total number of items processed.
  ///
  /// # Returns
  ///
  /// The total count of items processed since metrics started.
  #[must_use]
  pub fn items_processed(&self) -> u64 {
    self.items_processed.load(Ordering::Relaxed)
  }

  /// Gets the total number of items produced.
  ///
  /// # Returns
  ///
  /// The total count of items produced since metrics started.
  #[must_use]
  pub fn items_produced(&self) -> u64 {
    self.items_produced.load(Ordering::Relaxed)
  }

  /// Gets the total number of items consumed.
  ///
  /// # Returns
  ///
  /// The total count of items consumed since metrics started.
  #[must_use]
  pub fn items_consumed(&self) -> u64 {
    self.items_consumed.load(Ordering::Relaxed)
  }

  /// Calculates the items processed per second.
  ///
  /// # Returns
  ///
  /// The throughput in items per second, or 0.0 if less than a second has elapsed.
  #[must_use]
  pub fn items_per_second(&self) -> f64 {
    let elapsed = self.start_time.elapsed();
    if elapsed.as_secs() == 0 && elapsed.subsec_nanos() == 0 {
      return 0.0;
    }
    let items = self.items_processed.load(Ordering::Relaxed) as f64;
    items / elapsed.as_secs_f64()
  }
}

impl Default for ThroughputMetrics {
  fn default() -> Self {
    Self::new()
  }
}

/// Metrics for tracking latency percentiles.
///
/// Latency metrics track the time taken to process items through different
/// stages of the pipeline, providing percentile-based statistics (p50, p95, p99).
///
/// # Implementation Note
///
/// This implementation uses a simplified approach with a fixed-size histogram
/// for efficient percentile calculation. For production use, consider using
/// a library like `metrics` or `prometheus` for more sophisticated percentile tracking.
#[derive(Debug)]
pub struct LatencyMetrics {
  /// Sum of all recorded latencies (for average calculation).
  latency_sum: Arc<AtomicU64>,
  /// Count of latency measurements.
  latency_count: Arc<AtomicU64>,
  /// Minimum latency observed (in nanoseconds).
  latency_min: Arc<AtomicU64>,
  /// Maximum latency observed (in nanoseconds).
  latency_max: Arc<AtomicU64>,
  /// Latency buckets for percentile calculation (in nanoseconds).
  /// Buckets: <1ms, <10ms, <100ms, <1s, <10s, >=10s
  latency_buckets: Arc<[AtomicU64; 6]>,
}

impl LatencyMetrics {
  /// Creates a new latency metrics tracker.
  ///
  /// # Returns
  ///
  /// A new `LatencyMetrics` instance initialized to zero.
  #[must_use]
  pub fn new() -> Self {
    Self {
      latency_sum: Arc::new(AtomicU64::new(0)),
      latency_count: Arc::new(AtomicU64::new(0)),
      latency_min: Arc::new(AtomicU64::new(u64::MAX)),
      latency_max: Arc::new(AtomicU64::new(0)),
      latency_buckets: Arc::new([
        AtomicU64::new(0), // <1ms
        AtomicU64::new(0), // <10ms
        AtomicU64::new(0), // <100ms
        AtomicU64::new(0), // <1s
        AtomicU64::new(0), // <10s
        AtomicU64::new(0), // >=10s
      ]),
    }
  }

  /// Records a latency measurement.
  ///
  /// # Arguments
  ///
  /// * `latency` - The latency duration to record.
  pub fn record_latency(&self, latency: Duration) {
    let nanos = latency.as_nanos() as u64;

    // Update sum and count
    self.latency_sum.fetch_add(nanos, Ordering::Relaxed);
    self.latency_count.fetch_add(1, Ordering::Relaxed);

    // Update min and max
    let mut current_min = self.latency_min.load(Ordering::Relaxed);
    while nanos < current_min {
      match self.latency_min.compare_exchange_weak(
        current_min,
        nanos,
        Ordering::Relaxed,
        Ordering::Relaxed,
      ) {
        Ok(_) => break,
        Err(x) => current_min = x,
      }
    }

    let mut current_max = self.latency_max.load(Ordering::Relaxed);
    while nanos > current_max {
      match self.latency_max.compare_exchange_weak(
        current_max,
        nanos,
        Ordering::Relaxed,
        Ordering::Relaxed,
      ) {
        Ok(_) => break,
        Err(x) => current_max = x,
      }
    }

    // Update buckets
    let bucket_index = if nanos < 1_000_000 {
      0 // <1ms
    } else if nanos < 10_000_000 {
      1 // <10ms
    } else if nanos < 100_000_000 {
      2 // <100ms
    } else if nanos < 1_000_000_000 {
      3 // <1s
    } else if nanos < 10_000_000_000 {
      4 // <10s
    } else {
      5 // >=10s
    };
    self.latency_buckets[bucket_index].fetch_add(1, Ordering::Relaxed);
  }

  /// Gets the average latency.
  ///
  /// # Returns
  ///
  /// The average latency, or `None` if no measurements have been recorded.
  #[must_use]
  pub fn latency_avg(&self) -> Option<Duration> {
    let count = self.latency_count.load(Ordering::Relaxed);
    if count == 0 {
      return None;
    }
    let sum = self.latency_sum.load(Ordering::Relaxed);
    Some(Duration::from_nanos(sum / count))
  }

  /// Gets the minimum latency observed.
  ///
  /// # Returns
  ///
  /// The minimum latency, or `None` if no measurements have been recorded.
  #[must_use]
  pub fn latency_min(&self) -> Option<Duration> {
    let min = self.latency_min.load(Ordering::Relaxed);
    if min == u64::MAX {
      None
    } else {
      Some(Duration::from_nanos(min))
    }
  }

  /// Gets the maximum latency observed.
  ///
  /// # Returns
  ///
  /// The maximum latency, or `None` if no measurements have been recorded.
  #[must_use]
  pub fn latency_max(&self) -> Option<Duration> {
    let max = self.latency_max.load(Ordering::Relaxed);
    if max == 0 {
      None
    } else {
      Some(Duration::from_nanos(max))
    }
  }

  /// Estimates the 50th percentile (median) latency.
  ///
  /// # Returns
  ///
  /// An estimate of the p50 latency, or `None` if no measurements recorded.
  ///
  /// # Note
  ///
  /// This is a simplified estimation based on buckets. For precise percentiles,
  /// consider using a dedicated histogram library.
  #[must_use]
  pub fn latency_p50(&self) -> Option<Duration> {
    self.percentile(0.50)
  }

  /// Estimates the 95th percentile latency.
  ///
  /// # Returns
  ///
  /// An estimate of the p95 latency, or `None` if no measurements recorded.
  #[must_use]
  pub fn latency_p95(&self) -> Option<Duration> {
    self.percentile(0.95)
  }

  /// Estimates the 99th percentile latency.
  ///
  /// # Returns
  ///
  /// An estimate of the p99 latency, or `None` if no measurements recorded.
  #[must_use]
  pub fn latency_p99(&self) -> Option<Duration> {
    self.percentile(0.99)
  }

  /// Estimates a percentile latency.
  ///
  /// # Arguments
  ///
  /// * `percentile` - The percentile to estimate (0.0 to 1.0).
  ///
  /// # Returns
  ///
  /// An estimate of the requested percentile latency.
  #[must_use]
  fn percentile(&self, percentile: f64) -> Option<Duration> {
    let total = self.latency_count.load(Ordering::Relaxed);
    if total == 0 {
      return None;
    }

    let target = (total as f64 * percentile) as u64;
    let mut cumulative = 0u64;

    // Bucket thresholds in nanoseconds
    let thresholds = [
      1_000_000,      // 1ms
      10_000_000,     // 10ms
      100_000_000,    // 100ms
      1_000_000_000,  // 1s
      10_000_000_000, // 10s
    ];

    for (i, bucket) in self.latency_buckets.iter().take(5).enumerate() {
      cumulative += bucket.load(Ordering::Relaxed);
      if cumulative >= target {
        // Return midpoint of bucket
        let lower = if i == 0 { 0 } else { thresholds[i - 1] };
        let upper = thresholds[i];
        let midpoint = (lower + upper) / 2;
        return Some(Duration::from_nanos(midpoint));
      }
    }

    // If we get here, the percentile is in the last bucket (>=10s)
    Some(Duration::from_secs(10))
  }

  /// Gets the total number of latency measurements.
  ///
  /// # Returns
  ///
  /// The count of latency measurements recorded.
  #[must_use]
  pub fn latency_count(&self) -> u64 {
    self.latency_count.load(Ordering::Relaxed)
  }
}

impl Clone for LatencyMetrics {
  fn clone(&self) -> Self {
    Self {
      latency_sum: Arc::clone(&self.latency_sum),
      latency_count: Arc::clone(&self.latency_count),
      latency_min: Arc::clone(&self.latency_min),
      latency_max: Arc::clone(&self.latency_max),
      latency_buckets: Arc::clone(&self.latency_buckets),
    }
  }
}

impl Default for LatencyMetrics {
  fn default() -> Self {
    Self::new()
  }
}

/// Metrics for tracking errors.
///
/// Error metrics track the rate and types of errors that occur during pipeline execution.
#[derive(Debug)]
pub struct ErrorMetrics {
  /// Total number of errors encountered.
  total_errors: Arc<AtomicU64>,
  /// Number of errors by type (error message prefix).
  /// Uses RwLock for thread-safe concurrent access.
  errors_by_type: Arc<std::sync::RwLock<std::collections::HashMap<String, Arc<AtomicU64>>>>,
  /// Number of errors that resulted in stopping the pipeline.
  fatal_errors: Arc<AtomicU64>,
  /// Number of errors that were skipped.
  skipped_errors: Arc<AtomicU64>,
  /// Number of errors that were retried.
  retried_errors: Arc<AtomicU64>,
}

impl ErrorMetrics {
  /// Creates a new error metrics tracker.
  ///
  /// # Returns
  ///
  /// A new `ErrorMetrics` instance initialized to zero.
  #[must_use]
  pub fn new() -> Self {
    Self {
      total_errors: Arc::new(AtomicU64::new(0)),
      errors_by_type: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
      fatal_errors: Arc::new(AtomicU64::new(0)),
      skipped_errors: Arc::new(AtomicU64::new(0)),
      retried_errors: Arc::new(AtomicU64::new(0)),
    }
  }

  /// Records an error occurrence.
  ///
  /// # Arguments
  ///
  /// * `error_type` - A string identifier for the error type (e.g., "ParseError", "NetworkError").
  pub fn record_error(&self, error_type: &str) {
    self.total_errors.fetch_add(1, Ordering::Relaxed);

    // Try to get existing counter (read lock)
    {
      let map = self.errors_by_type.read().expect(
        "Failed to acquire read lock on errors_by_type. This indicates a poisoned lock, \
         which should not occur in normal operation.",
      );
      if let Some(counter) = map.get(error_type) {
        counter.fetch_add(1, Ordering::Relaxed);
        return;
      }
    }

    // Need to create new counter (write lock)
    let mut map = self.errors_by_type.write().expect(
      "Failed to acquire write lock on errors_by_type. This indicates a poisoned lock, \
       which should not occur in normal operation.",
    );

    // Double-check after acquiring write lock (another thread might have inserted)
    if let Some(counter) = map.get(error_type) {
      counter.fetch_add(1, Ordering::Relaxed);
      return;
    }

    // Create and insert new counter
    let counter = Arc::new(AtomicU64::new(1));
    map.insert(error_type.to_string(), Arc::clone(&counter));
  }

  /// Records a fatal error (pipeline stopped).
  pub fn record_fatal_error(&self) {
    self.fatal_errors.fetch_add(1, Ordering::Relaxed);
  }

  /// Records a skipped error.
  pub fn record_skipped_error(&self) {
    self.skipped_errors.fetch_add(1, Ordering::Relaxed);
  }

  /// Records a retried error.
  pub fn record_retried_error(&self) {
    self.retried_errors.fetch_add(1, Ordering::Relaxed);
  }

  /// Gets the total number of errors.
  ///
  /// # Returns
  ///
  /// The total count of errors recorded.
  #[must_use]
  pub fn total_errors(&self) -> u64 {
    self.total_errors.load(Ordering::Relaxed)
  }

  /// Gets the number of fatal errors.
  ///
  /// # Returns
  ///
  /// The count of fatal errors.
  #[must_use]
  pub fn fatal_errors(&self) -> u64 {
    self.fatal_errors.load(Ordering::Relaxed)
  }

  /// Gets the number of skipped errors.
  ///
  /// # Returns
  ///
  /// The count of skipped errors.
  #[must_use]
  pub fn skipped_errors(&self) -> u64 {
    self.skipped_errors.load(Ordering::Relaxed)
  }

  /// Gets the number of retried errors.
  ///
  /// # Returns
  ///
  /// The count of retried errors.
  #[must_use]
  pub fn retried_errors(&self) -> u64 {
    self.retried_errors.load(Ordering::Relaxed)
  }

  /// Gets error counts by type.
  ///
  /// # Returns
  ///
  /// A map of error type to count.
  #[must_use]
  pub fn errors_by_type(&self) -> std::collections::HashMap<String, u64> {
    let map = self.errors_by_type.read().expect(
      "Failed to acquire read lock on errors_by_type. This indicates a poisoned lock, \
       which should not occur in normal operation.",
    );
    map
      .iter()
      .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
      .collect()
  }

  /// Calculates the error rate (errors per second).
  ///
  /// # Arguments
  ///
  /// * `elapsed` - The elapsed time since metrics started.
  ///
  /// # Returns
  ///
  /// The error rate in errors per second.
  #[must_use]
  pub fn error_rate(&self, elapsed: Duration) -> f64 {
    if elapsed.as_secs() == 0 && elapsed.subsec_nanos() == 0 {
      return 0.0;
    }
    let errors = self.total_errors.load(Ordering::Relaxed) as f64;
    errors / elapsed.as_secs_f64()
  }
}

impl Clone for ErrorMetrics {
  fn clone(&self) -> Self {
    Self {
      total_errors: Arc::clone(&self.total_errors),
      errors_by_type: Arc::clone(&self.errors_by_type),
      fatal_errors: Arc::clone(&self.fatal_errors),
      skipped_errors: Arc::clone(&self.skipped_errors),
      retried_errors: Arc::clone(&self.retried_errors),
    }
  }
}

impl Default for ErrorMetrics {
  fn default() -> Self {
    Self::new()
  }
}

/// Backpressure level indicator.
///
/// Backpressure indicates when downstream components cannot keep up with upstream producers,
/// causing items to queue up in buffers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackpressureLevel {
  /// No backpressure - pipeline is running smoothly.
  None = 0,
  /// Low backpressure - some buffering is occurring but manageable.
  Low = 1,
  /// Medium backpressure - significant buffering, may impact performance.
  Medium = 2,
  /// High backpressure - severe buffering, pipeline may be blocked.
  High = 3,
}

impl BackpressureLevel {
  /// Gets a human-readable string representation.
  ///
  /// # Returns
  ///
  /// A string describing the backpressure level.
  #[must_use]
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::None => "none",
      Self::Low => "low",
      Self::Medium => "medium",
      Self::High => "high",
    }
  }
}

/// Comprehensive pipeline metrics.
///
/// This structure aggregates all metric types for a complete view of pipeline performance.
///
/// # Example
///
/// ```rust
/// use streamweave::metrics::PipelineMetrics;
///
/// let metrics = PipelineMetrics::new("my-pipeline");
/// metrics.throughput().increment_items_processed(100);
/// metrics.record_latency(std::time::Duration::from_millis(50));
/// ```
#[derive(Debug, Clone)]
pub struct PipelineMetrics {
  /// Pipeline name/identifier.
  name: String,
  /// Throughput metrics.
  throughput: ThroughputMetrics,
  /// Latency metrics.
  latency: LatencyMetrics,
  /// Error metrics.
  errors: ErrorMetrics,
  /// Current backpressure level.
  backpressure: Arc<AtomicU64>, // Stored as BackpressureLevel as u8
  /// Start time for the pipeline.
  start_time: Arc<Instant>,
}

impl PipelineMetrics {
  /// Creates new pipeline metrics.
  ///
  /// # Arguments
  ///
  /// * `name` - Pipeline identifier/name.
  ///
  /// # Returns
  ///
  /// A new `PipelineMetrics` instance.
  #[must_use]
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      throughput: ThroughputMetrics::new(),
      latency: LatencyMetrics::new(),
      errors: ErrorMetrics::new(),
      backpressure: Arc::new(AtomicU64::new(BackpressureLevel::None as u64)),
      start_time: Arc::new(Instant::now()),
    }
  }

  /// Gets the pipeline name.
  ///
  /// # Returns
  ///
  /// The pipeline identifier.
  #[must_use]
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Gets throughput metrics.
  ///
  /// # Returns
  ///
  /// A reference to the throughput metrics.
  #[must_use]
  pub fn throughput(&self) -> &ThroughputMetrics {
    &self.throughput
  }

  /// Gets latency metrics.
  ///
  /// # Returns
  ///
  /// A reference to the latency metrics.
  #[must_use]
  pub fn latency(&self) -> &LatencyMetrics {
    &self.latency
  }

  /// Gets error metrics.
  ///
  /// # Returns
  ///
  /// A reference to the error metrics.
  #[must_use]
  pub fn errors(&self) -> &ErrorMetrics {
    &self.errors
  }

  /// Records a latency measurement.
  ///
  /// # Arguments
  ///
  /// * `latency` - The latency duration to record.
  pub fn record_latency(&self, latency: Duration) {
    self.latency.record_latency(latency);
  }

  /// Sets the current backpressure level.
  ///
  /// # Arguments
  ///
  /// * `level` - The backpressure level.
  pub fn set_backpressure(&self, level: BackpressureLevel) {
    self.backpressure.store(level as u64, Ordering::Relaxed);
  }

  /// Gets the current backpressure level.
  ///
  /// # Returns
  ///
  /// The current backpressure level.
  ///
  /// # Safety
  ///
  /// If an invalid value is stored, this will default to `BackpressureLevel::None`.
  #[must_use]
  pub fn backpressure(&self) -> BackpressureLevel {
    let value = self.backpressure.load(Ordering::Relaxed) as u8;
    // Safe conversion: match on the value to ensure we only return valid enum variants
    match value {
      0 => BackpressureLevel::None,
      1 => BackpressureLevel::Low,
      2 => BackpressureLevel::Medium,
      3 => BackpressureLevel::High,
      _ => {
        // Invalid value stored - default to None to avoid undefined behavior
        // This should never happen in practice, but provides a safe fallback
        BackpressureLevel::None
      }
    }
  }

  /// Gets the elapsed time since metrics started.
  ///
  /// # Returns
  ///
  /// The elapsed duration.
  #[must_use]
  pub fn elapsed(&self) -> Duration {
    self.start_time.elapsed()
  }
}

impl Default for PipelineMetrics {
  fn default() -> Self {
    Self::new("unnamed-pipeline")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_throughput_metrics() {
    let metrics = ThroughputMetrics::new();
    assert_eq!(metrics.items_processed(), 0);

    metrics.increment_items_processed(10);
    assert_eq!(metrics.items_processed(), 10);

    metrics.increment_items_produced(5);
    assert_eq!(metrics.items_produced(), 5);

    metrics.increment_items_consumed(8);
    assert_eq!(metrics.items_consumed(), 8);
  }

  #[test]
  fn test_latency_metrics() {
    let metrics = LatencyMetrics::new();
    assert!(metrics.latency_avg().is_none());

    metrics.record_latency(Duration::from_millis(10));
    metrics.record_latency(Duration::from_millis(20));
    metrics.record_latency(Duration::from_millis(30));

    let avg = metrics.latency_avg().unwrap();
    assert!(avg >= Duration::from_millis(15) && avg <= Duration::from_millis(25));

    let min = metrics.latency_min().unwrap();
    assert!(min <= Duration::from_millis(15));

    let max = metrics.latency_max().unwrap();
    assert!(max >= Duration::from_millis(25));

    assert!(metrics.latency_p50().is_some());
    assert!(metrics.latency_p95().is_some());
    assert!(metrics.latency_p99().is_some());
  }

  #[test]
  fn test_error_metrics() {
    let metrics = ErrorMetrics::new();
    assert_eq!(metrics.total_errors(), 0);

    metrics.record_error("ParseError");
    metrics.record_error("NetworkError");
    metrics.record_error("ParseError");

    assert_eq!(metrics.total_errors(), 3);

    let by_type = metrics.errors_by_type();
    assert_eq!(by_type.get("ParseError"), Some(&2));
    assert_eq!(by_type.get("NetworkError"), Some(&1));

    metrics.record_fatal_error();
    assert_eq!(metrics.fatal_errors(), 1);
  }

  #[test]
  fn test_pipeline_metrics() {
    let metrics = PipelineMetrics::new("test-pipeline");
    assert_eq!(metrics.name(), "test-pipeline");

    metrics.throughput().increment_items_processed(100);
    assert_eq!(metrics.throughput().items_processed(), 100);

    metrics.record_latency(Duration::from_millis(50));
    assert!(metrics.latency().latency_avg().is_some());

    metrics.set_backpressure(BackpressureLevel::Medium);
    assert_eq!(metrics.backpressure(), BackpressureLevel::Medium);
  }
}
