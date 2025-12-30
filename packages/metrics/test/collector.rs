use std::time::Duration;
use streamweave_metrics::MetricsCollector;

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

#[test]
fn test_metrics_collector_default() {
  let collector = MetricsCollector::default();
  let handle = collector.metrics();
  assert_eq!(handle.pipeline_name(), "unnamed-pipeline");
}

#[test]
fn test_metrics_collector_clone() {
  let collector = MetricsCollector::new("test");
  let handle1 = collector.metrics();

  let cloned = collector.clone();
  let handle2 = cloned.metrics();

  // Both should access the same underlying metrics
  handle1.metrics().throughput().increment_items_processed(10);
  assert_eq!(handle2.metrics().throughput().items_processed(), 10);
}

#[test]
fn test_metrics_collector_pipeline_metrics() {
  let collector = MetricsCollector::new("test");
  let pipeline_metrics = collector.pipeline_metrics();

  assert_eq!(pipeline_metrics.name(), "test");
  pipeline_metrics.throughput().increment_items_processed(5);
  assert_eq!(pipeline_metrics.throughput().items_processed(), 5);
}

#[test]
fn test_metrics_handle_all_methods() {
  let collector = MetricsCollector::new("test");
  let handle = collector.metrics();

  // Test all accessor methods
  assert_eq!(handle.pipeline_name(), "test");

  handle.metrics().throughput().increment_items_processed(10);
  assert_eq!(handle.throughput().items_processed(), 10);

  handle
    .metrics()
    .latency()
    .record_latency(Duration::from_millis(50));
  assert!(handle.latency().latency_avg().is_some());

  handle.metrics().errors().record_error("TestError");
  assert_eq!(handle.errors().total_errors(), 1);

  use streamweave_metrics::BackpressureLevel;
  handle.metrics().set_backpressure(BackpressureLevel::High);
  assert_eq!(handle.backpressure() as u8, 3);

  let elapsed = handle.elapsed();
  assert!(elapsed.as_secs() >= 0);
}

#[test]
fn test_metrics_handle_clone() {
  let collector = MetricsCollector::new("test");
  let handle1 = collector.metrics();
  let handle2 = handle1.clone();

  handle1.metrics().throughput().increment_items_processed(10);
  assert_eq!(handle2.metrics().throughput().items_processed(), 10);
}
