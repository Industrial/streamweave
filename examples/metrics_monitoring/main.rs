mod pipeline;

use pipeline::run_metrics_example;

/// Example demonstrating StreamWeave metrics collection.
///
/// This example shows how to:
/// - Create a metrics collector
/// - Track throughput, latency, and errors
/// - Monitor backpressure indicators
/// - Access metrics during and after pipeline execution
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 StreamWeave Metrics and Monitoring Example");
  println!("==============================================");
  println!("This example demonstrates comprehensive metrics collection:");
  println!("1. Throughput metrics (items/second)");
  println!("2. Latency metrics (p50, p95, p99)");
  println!("3. Error metrics (rates and types)");
  println!("4. Backpressure indicators");
  println!();

  run_metrics_example().await?;

  println!();
  println!("✅ Metrics example completed successfully!");
  println!("Key Features Demonstrated:");
  println!("• MetricsCollector: Tracks pipeline metrics");
  println!("• ThroughputMetrics: Items processed per second");
  println!("• LatencyMetrics: Percentile-based latency tracking");
  println!("• ErrorMetrics: Error rates and categorization");
  println!("• BackpressureLevel: Pipeline congestion indicators");

  Ok(())
}
