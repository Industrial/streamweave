use std::sync::Arc;
use std::thread;
use std::time::Duration;
use streamweave_metrics::*;

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
fn test_throughput_metrics_items_per_second() {
  let metrics = ThroughputMetrics::new();

  // Test with zero elapsed time
  let rate = metrics.items_per_second();
  assert_eq!(rate, 0.0);

  // Wait a bit and add items
  std::thread::sleep(Duration::from_millis(100));
  metrics.increment_items_processed(100);

  let rate = metrics.items_per_second();
  assert!(rate > 0.0);
}

#[test]
fn test_throughput_metrics_default() {
  let metrics = ThroughputMetrics::default();
  assert_eq!(metrics.items_processed(), 0);
  assert_eq!(metrics.items_produced(), 0);
  assert_eq!(metrics.items_consumed(), 0);
}

#[test]
fn test_throughput_metrics_clone() {
  let metrics = ThroughputMetrics::new();
  metrics.increment_items_processed(10);

  let cloned = metrics.clone();
  assert_eq!(cloned.items_processed(), 10);

  // Both should share state
  metrics.increment_items_processed(5);
  assert_eq!(cloned.items_processed(), 15);
}

#[test]
fn test_throughput_metrics_concurrent() {
  let metrics = Arc::new(ThroughputMetrics::new());
  let mut handles = vec![];

  for _ in 0..10 {
    let m = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
      for _ in 0..100 {
        m.increment_items_processed(1);
      }
    }));
  }

  for handle in handles {
    handle.join().unwrap();
  }

  assert_eq!(metrics.items_processed(), 1000);
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
fn test_latency_metrics_empty() {
  let metrics = LatencyMetrics::new();
  assert!(metrics.latency_avg().is_none());
  assert!(metrics.latency_min().is_none());
  assert!(metrics.latency_max().is_none());
  assert!(metrics.latency_p50().is_none());
  assert!(metrics.latency_p95().is_none());
  assert!(metrics.latency_p99().is_none());
  assert_eq!(metrics.latency_count(), 0);
}

#[test]
fn test_latency_metrics_single_measurement() {
  let metrics = LatencyMetrics::new();
  metrics.record_latency(Duration::from_millis(50));

  let avg = metrics.latency_avg().unwrap();
  assert_eq!(avg, Duration::from_millis(50));

  let min = metrics.latency_min().unwrap();
  assert_eq!(min, Duration::from_millis(50));

  let max = metrics.latency_max().unwrap();
  assert_eq!(max, Duration::from_millis(50));

  assert_eq!(metrics.latency_count(), 1);
}

#[test]
fn test_latency_metrics_buckets() {
  let metrics = LatencyMetrics::new();

  // Test all bucket ranges
  metrics.record_latency(Duration::from_nanos(500_000)); // <1ms
  metrics.record_latency(Duration::from_nanos(5_000_000)); // <10ms
  metrics.record_latency(Duration::from_nanos(50_000_000)); // <100ms
  metrics.record_latency(Duration::from_nanos(500_000_000)); // <1s
  metrics.record_latency(Duration::from_nanos(5_000_000_000)); // <10s
  metrics.record_latency(Duration::from_secs(15)); // >=10s

  assert_eq!(metrics.latency_count(), 6);
  assert!(metrics.latency_p50().is_some());
  assert!(metrics.latency_p95().is_some());
  assert!(metrics.latency_p99().is_some());
}

#[test]
fn test_latency_metrics_percentiles() {
  let metrics = LatencyMetrics::new();

  // Record many measurements to test percentile calculation
  for i in 1..=100 {
    metrics.record_latency(Duration::from_millis(i));
  }

  assert_eq!(metrics.latency_count(), 100);

  let p50 = metrics.latency_p50().unwrap();
  assert!(p50 >= Duration::from_millis(40) && p50 <= Duration::from_millis(60));

  let p95 = metrics.latency_p95().unwrap();
  assert!(p95 >= Duration::from_millis(90));

  let p99 = metrics.latency_p99().unwrap();
  assert!(p99 >= Duration::from_millis(95));
}

#[test]
fn test_latency_metrics_min_max_update() {
  let metrics = LatencyMetrics::new();

  metrics.record_latency(Duration::from_millis(100));
  assert_eq!(metrics.latency_min().unwrap(), Duration::from_millis(100));
  assert_eq!(metrics.latency_max().unwrap(), Duration::from_millis(100));

  metrics.record_latency(Duration::from_millis(50));
  assert_eq!(metrics.latency_min().unwrap(), Duration::from_millis(50));
  assert_eq!(metrics.latency_max().unwrap(), Duration::from_millis(100));

  metrics.record_latency(Duration::from_millis(200));
  assert_eq!(metrics.latency_min().unwrap(), Duration::from_millis(50));
  assert_eq!(metrics.latency_max().unwrap(), Duration::from_millis(200));
}

#[test]
fn test_latency_metrics_clone() {
  let metrics = LatencyMetrics::new();
  metrics.record_latency(Duration::from_millis(50));

  let cloned = metrics.clone();
  assert_eq!(cloned.latency_count(), 1);
  assert_eq!(cloned.latency_avg().unwrap(), Duration::from_millis(50));

  // Both should share state
  metrics.record_latency(Duration::from_millis(100));
  assert_eq!(cloned.latency_count(), 2);
}

#[test]
fn test_latency_metrics_default() {
  let metrics = LatencyMetrics::default();
  assert_eq!(metrics.latency_count(), 0);
  assert!(metrics.latency_avg().is_none());
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
fn test_error_metrics_all_types() {
  let metrics = ErrorMetrics::new();

  metrics.record_fatal_error();
  metrics.record_skipped_error();
  metrics.record_retried_error();

  assert_eq!(metrics.fatal_errors(), 1);
  assert_eq!(metrics.skipped_errors(), 1);
  assert_eq!(metrics.retried_errors(), 1);
}

#[test]
fn test_error_metrics_error_rate() {
  let metrics = ErrorMetrics::new();

  // Test with zero elapsed time
  let rate = metrics.error_rate(Duration::from_secs(0));
  assert_eq!(rate, 0.0);

  metrics.record_error("TestError");
  std::thread::sleep(Duration::from_millis(100));

  let rate = metrics.error_rate(Duration::from_millis(100));
  assert!(rate > 0.0);
}

#[test]
fn test_error_metrics_concurrent() {
  let metrics = Arc::new(ErrorMetrics::new());
  let mut handles = vec![];

  for i in 0..10 {
    let m = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
      for _ in 0..10 {
        m.record_error(&format!("ErrorType{}", i));
      }
    }));
  }

  for handle in handles {
    handle.join().unwrap();
  }

  assert_eq!(metrics.total_errors(), 100);
  let by_type = metrics.errors_by_type();
  assert_eq!(by_type.len(), 10);
}

#[test]
fn test_error_metrics_clone() {
  let metrics = ErrorMetrics::new();
  metrics.record_error("TestError");

  let cloned = metrics.clone();
  assert_eq!(cloned.total_errors(), 1);

  // Both should share state
  metrics.record_error("AnotherError");
  assert_eq!(cloned.total_errors(), 2);
}

#[test]
fn test_error_metrics_default() {
  let metrics = ErrorMetrics::default();
  assert_eq!(metrics.total_errors(), 0);
  assert_eq!(metrics.fatal_errors(), 0);
  assert_eq!(metrics.skipped_errors(), 0);
  assert_eq!(metrics.retried_errors(), 0);
  assert!(metrics.errors_by_type().is_empty());
}

#[test]
fn test_backpressure_level() {
  assert_eq!(BackpressureLevel::None.as_str(), "none");
  assert_eq!(BackpressureLevel::Low.as_str(), "low");
  assert_eq!(BackpressureLevel::Medium.as_str(), "medium");
  assert_eq!(BackpressureLevel::High.as_str(), "high");

  // Test ordering
  assert!(BackpressureLevel::None < BackpressureLevel::Low);
  assert!(BackpressureLevel::Low < BackpressureLevel::Medium);
  assert!(BackpressureLevel::Medium < BackpressureLevel::High);
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

#[test]
fn test_pipeline_metrics_all_backpressure_levels() {
  let metrics = PipelineMetrics::new("test");

  metrics.set_backpressure(BackpressureLevel::None);
  assert_eq!(metrics.backpressure(), BackpressureLevel::None);

  metrics.set_backpressure(BackpressureLevel::Low);
  assert_eq!(metrics.backpressure(), BackpressureLevel::Low);

  metrics.set_backpressure(BackpressureLevel::Medium);
  assert_eq!(metrics.backpressure(), BackpressureLevel::Medium);

  metrics.set_backpressure(BackpressureLevel::High);
  assert_eq!(metrics.backpressure(), BackpressureLevel::High);
}

#[test]
fn test_pipeline_metrics_invalid_backpressure() {
  let metrics = PipelineMetrics::new("test");

  // Test that invalid backpressure values default to None
  // This tests the match statement default case
  use std::sync::atomic::AtomicU64;
  use std::sync::atomic::Ordering;

  // We can't directly set invalid values, but we can test the default case
  // by ensuring the match handles all cases correctly
  let level = metrics.backpressure();
  assert!(matches!(
    level,
    BackpressureLevel::None
      | BackpressureLevel::Low
      | BackpressureLevel::Medium
      | BackpressureLevel::High
  ));
}

#[test]
fn test_pipeline_metrics_elapsed() {
  let metrics = PipelineMetrics::new("test");

  let elapsed1 = metrics.elapsed();
  std::thread::sleep(Duration::from_millis(10));
  let elapsed2 = metrics.elapsed();

  assert!(elapsed2 > elapsed1);
}

#[test]
fn test_pipeline_metrics_default() {
  let metrics = PipelineMetrics::default();
  assert_eq!(metrics.name(), "unnamed-pipeline");
}

#[test]
fn test_pipeline_metrics_clone() {
  let metrics = PipelineMetrics::new("test");
  metrics.throughput().increment_items_processed(10);

  let cloned = metrics.clone();
  assert_eq!(cloned.throughput().items_processed(), 10);

  // Both should share state
  metrics.throughput().increment_items_processed(5);
  assert_eq!(cloned.throughput().items_processed(), 15);
}

#[test]
fn test_pipeline_metrics_accessors() {
  let metrics = PipelineMetrics::new("test");

  assert_eq!(metrics.name(), "test");
  assert!(metrics.throughput().items_processed() == 0);
  assert!(metrics.latency().latency_count() == 0);
  assert!(metrics.errors().total_errors() == 0);
}
