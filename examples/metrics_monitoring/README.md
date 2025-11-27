# StreamWeave Metrics and Monitoring Example

This example demonstrates comprehensive metrics collection and monitoring capabilities in StreamWeave pipelines.

## Overview

The metrics example shows how to:
- Create and use a `MetricsCollector` to track pipeline performance
- Monitor throughput (items per second)
- Track latency with percentile metrics (p50, p95, p99)
- Collect error metrics (rates and types)
- Monitor backpressure indicators
- Access metrics during and after pipeline execution

## Running the Example

```bash
cargo run --example metrics_monitoring
```

## What It Demonstrates

### 1. Throughput Metrics
- Items processed per second
- Items produced vs consumed
- Total items processed

### 2. Latency Metrics
- Average, minimum, and maximum latency
- Percentile-based latency (p50, p95, p99)
- Total latency measurements

### 3. Error Metrics
- Total error count
- Errors by type (ParseError, NetworkError, etc.)
- Error categories (fatal, skipped, retried)
- Error rate (errors per second)

### 4. Backpressure Indicators
- Current backpressure level (None, Low, Medium, High)
- Backpressure level tracking

## Code Structure

- `main.rs`: Entry point and example orchestration
- `pipeline.rs`: Pipeline implementation with metrics collection

## Key Concepts

### MetricsCollector
The `MetricsCollector` is the main entry point for metrics collection:

```rust
use streamweave::metrics::MetricsCollector;

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();
```

### Accessing Metrics
Metrics can be accessed at any time through the `MetricsHandle`:

```rust
let throughput = metrics_handle.throughput().items_per_second();
let latency_p95 = metrics_handle.latency().latency_p95();
let error_count = metrics_handle.errors().total_errors();
```

### Recording Metrics
Metrics are recorded during pipeline execution:

```rust
metrics.throughput().increment_items_processed(count);
metrics.record_latency(duration);
metrics.errors().record_error("ErrorType");
metrics.set_backpressure(BackpressureLevel::Medium);
```

## Expected Output

The example will output:
- Pipeline execution summary
- Comprehensive metrics breakdown
- Error metrics demonstration
- Backpressure level tracking

## Integration with Pipelines

While this example demonstrates manual metrics collection, in production you would typically:
- Integrate metrics collection into pipeline execution
- Automatically track metrics for each pipeline stage
- Export metrics to monitoring systems (Prometheus, OpenTelemetry)
- Use metrics for alerting and performance optimization

## Next Steps

- See Task 11.2 for OpenTelemetry integration
- See Task 11.3 for Prometheus export
- See Task 11.4 for health check endpoints

