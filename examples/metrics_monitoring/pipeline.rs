//! Pipeline implementation for metrics example.
//!
//! This module demonstrates how to use metrics collection with StreamWeave pipelines.

use std::time::{Duration, Instant};
use streamweave::metrics::{BackpressureLevel, MetricsCollector};
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  pipeline::PipelineBuilder,
  producers::range::range_producer::RangeProducer,
  transformers::{
    filter::filter_transformer::FilterTransformer, map::map_transformer::MapTransformer,
  },
};

/// Runs a pipeline with comprehensive metrics collection.
///
/// This function demonstrates:
/// - Creating a metrics collector
/// - Running a pipeline with metrics tracking
/// - Accessing metrics during and after execution
/// - Displaying metric values
pub async fn run_metrics_example() -> Result<(), Box<dyn std::error::Error>> {
  // Create a metrics collector for the pipeline
  let collector = MetricsCollector::new("metrics-demo-pipeline");
  let metrics_handle = collector.metrics();

  println!("Creating pipeline with metrics collection...");

  // Create a pipeline that processes numbers
  let pipeline = PipelineBuilder::new()
    .producer(RangeProducer::new(1, 1001, 1)) // Generate 1-1000
    .transformer(MapTransformer::new(|x: i32| {
      // Simple transformation (no sleep to avoid blocking async runtime)
      x * 2
    }))
    .transformer(FilterTransformer::new(|x: &i32| *x > 100))
    .consumer(VecConsumer::new());

  println!("Starting pipeline execution...");
  let start_time = Instant::now();

  // Track metrics during execution
  let metrics = metrics_handle.metrics();

  // Record start of processing
  metrics.throughput().increment_items_produced(1000);

  // Run the pipeline
  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();
  let execution_time = start_time.elapsed();

  // Record completion
  metrics
    .throughput()
    .increment_items_processed(results.len() as u64);
  metrics
    .throughput()
    .increment_items_consumed(results.len() as u64);

  // Record latency (simulated - in real usage, this would be tracked per item)
  metrics.record_latency(execution_time / (results.len() as u32));

  println!("Pipeline execution completed in {:?}", execution_time);
  println!("Processed {} items", results.len());
  println!();

  // Display metrics
  display_metrics(&metrics_handle, execution_time);

  // Demonstrate error metrics
  demonstrate_error_metrics(&collector).await?;

  // Demonstrate backpressure
  demonstrate_backpressure(&collector);

  Ok(())
}

/// Displays comprehensive metrics information.
fn display_metrics(handle: &streamweave::metrics::MetricsHandle, _execution_time: Duration) {
  let metrics = handle.metrics();

  println!("📈 Pipeline Metrics Summary");
  println!("===========================");
  println!("Pipeline Name: {}", handle.pipeline_name());
  println!("Elapsed Time: {:?}", handle.elapsed());
  println!();

  // Throughput metrics
  println!("🚀 Throughput Metrics:");
  let throughput = metrics.throughput();
  println!("  Items Processed: {}", throughput.items_processed());
  println!("  Items Produced: {}", throughput.items_produced());
  println!("  Items Consumed: {}", throughput.items_consumed());
  println!("  Items/Second: {:.2}", throughput.items_per_second());
  println!();

  // Latency metrics
  println!("⏱️  Latency Metrics:");
  let latency = metrics.latency();
  if let Some(avg) = latency.latency_avg() {
    println!("  Average: {:?}", avg);
  }
  if let Some(min) = latency.latency_min() {
    println!("  Minimum: {:?}", min);
  }
  if let Some(max) = latency.latency_max() {
    println!("  Maximum: {:?}", max);
  }
  if let Some(p50) = latency.latency_p50() {
    println!("  P50 (Median): {:?}", p50);
  }
  if let Some(p95) = latency.latency_p95() {
    println!("  P95: {:?}", p95);
  }
  if let Some(p99) = latency.latency_p99() {
    println!("  P99: {:?}", p99);
  }
  println!("  Total Measurements: {}", latency.latency_count());
  println!();

  // Error metrics
  println!("❌ Error Metrics:");
  let errors = metrics.errors();
  println!("  Total Errors: {}", errors.total_errors());
  println!("  Fatal Errors: {}", errors.fatal_errors());
  println!("  Skipped Errors: {}", errors.skipped_errors());
  println!("  Retried Errors: {}", errors.retried_errors());
  let error_rate = errors.error_rate(handle.elapsed());
  println!("  Error Rate: {:.2} errors/second", error_rate);

  let errors_by_type = errors.errors_by_type();
  if !errors_by_type.is_empty() {
    println!("  Errors by Type:");
    for (error_type, count) in errors_by_type {
      println!("    {}: {}", error_type, count);
    }
  }
  println!();

  // Backpressure
  println!("📊 Backpressure:");
  let backpressure = handle.backpressure();
  println!(
    "  Level: {} ({})",
    backpressure.as_str(),
    backpressure as u8
  );
  println!();
}

/// Demonstrates error metrics collection.
async fn demonstrate_error_metrics(
  collector: &MetricsCollector,
) -> Result<(), Box<dyn std::error::Error>> {
  println!("Demonstrating error metrics...");

  let metrics_handle = collector.metrics();
  let metrics = metrics_handle.metrics();

  // Record some example errors
  metrics.errors().record_error("ParseError");
  metrics.errors().record_error("NetworkError");
  metrics.errors().record_error("ParseError");
  metrics.errors().record_skipped_error();
  metrics.errors().record_retried_error();

  println!("Recorded example errors:");
  let errors_by_type = metrics.errors().errors_by_type();
  for (error_type, count) in errors_by_type {
    println!("  {}: {}", error_type, count);
  }
  println!();

  Ok(())
}

/// Demonstrates backpressure level tracking.
fn demonstrate_backpressure(collector: &MetricsCollector) {
  println!("Demonstrating backpressure tracking...");

  let metrics_handle = collector.metrics();
  let metrics = metrics_handle.metrics();

  // Set different backpressure levels
  for level in [
    BackpressureLevel::None,
    BackpressureLevel::Low,
    BackpressureLevel::Medium,
    BackpressureLevel::High,
  ] {
    metrics.set_backpressure(level);
    let current = metrics_handle.backpressure();
    println!(
      "  Set backpressure to: {} ({})",
      current.as_str(),
      current as u8
    );
  }
  println!();
}
