# streamweave-metrics

[![Crates.io](https://img.shields.io/crates/v/streamweave-metrics.svg)](https://crates.io/crates/streamweave-metrics)
[![Documentation](https://docs.rs/streamweave-metrics/badge.svg)](https://docs.rs/streamweave-metrics)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Metrics collection for StreamWeave**  
*Comprehensive observability through metrics, health checks, and monitoring integration.*

The `streamweave-metrics` package provides comprehensive metrics collection for StreamWeave pipelines. It enables tracking throughput, latency, errors, backpressure, and health status, with integration to Prometheus and OpenTelemetry.

## ✨ Key Features

- **Metrics Collection**: Automatic metrics collection for pipelines
- **Throughput Metrics**: Track items processed per second
- **Latency Metrics**: Track processing time percentiles (p50, p95, p99)
- **Error Metrics**: Track error rates and types
- **Backpressure Metrics**: Track pipeline congestion
- **Health Checks**: Health status tracking for components
- **Prometheus Integration**: Export metrics to Prometheus
- **OpenTelemetry Integration**: Export metrics via OpenTelemetry

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-metrics = { version = "0.3.0", features = ["prometheus", "opentelemetry"] }
```

## 🚀 Quick Start

### Basic Metrics Collection

```rust
use streamweave_metrics::{MetricsCollector, PipelineMetrics};

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();

// Metrics are automatically collected during pipeline execution
let throughput = metrics_handle.metrics().throughput();
let latency = metrics_handle.metrics().latency();
```

### Health Checks

```rust
use streamweave_metrics::health::{HealthCheck, ComponentHealth};

let mut health_check = HealthCheck::new("my-pipeline");

health_check.set_component_health("database", ComponentHealth::healthy());
health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));

let status = health_check.status();
let is_ready = health_check.is_ready();
```

### Prometheus Export

```rust
use streamweave_metrics::{MetricsCollector, PrometheusExporter};

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();

let exporter = PrometheusExporter::new("streamweave");
let prometheus_text = exporter.export(&metrics_handle.metrics());
```

## 📖 API Overview

### MetricsCollector

Collects metrics for pipelines:

```rust
pub struct MetricsCollector {
    // Internal state
}
```

**Key Methods:**
- `new(pipeline_name)` - Create collector
- `metrics()` - Get metrics handle
- `record_item_processed()` - Record processed item
- `record_latency(duration)` - Record processing latency
- `record_error(error)` - Record error

### PipelineMetrics

Pipeline metrics data:

```rust
pub struct PipelineMetrics {
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub errors: ErrorMetrics,
    pub backpressure: BackpressureLevel,
}
```

**Key Methods:**
- `new(pipeline_name)` - Create metrics
- `throughput()` - Get throughput metrics
- `latency()` - Get latency metrics
- `errors()` - Get error metrics
- `backpressure()` - Get backpressure level

### HealthCheck

Health check for pipelines:

```rust
pub struct HealthCheck {
    // Internal state
}
```

**Key Methods:**
- `new(pipeline_name)` - Create health check
- `set_component_health(name, health)` - Set component health
- `status()` - Get overall health status
- `is_ready()` - Check if ready
- `is_live()` - Check if live

### PrometheusExporter

Exports metrics to Prometheus:

```rust
pub struct PrometheusExporter {
    // Internal state
}
```

**Key Methods:**
- `new(prefix)` - Create exporter
- `export(metrics)` - Export metrics in Prometheus format

## 📚 Usage Examples

### Throughput Monitoring

Monitor throughput:

```rust
use streamweave_metrics::{MetricsCollector, ThroughputMetrics};

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();

// Access throughput metrics
let throughput = metrics_handle.metrics().throughput();
let items_per_sec = throughput.items_processed();
```

### Latency Tracking

Track latency:

```rust
use streamweave_metrics::{MetricsCollector, LatencyMetrics};
use std::time::Duration;

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();

// Record latency
collector.record_latency(Duration::from_millis(100));

// Access latency metrics
let latency = metrics_handle.metrics().latency();
let p95 = latency.latency_p95();
```

### Error Tracking

Track errors:

```rust
use streamweave_metrics::{MetricsCollector, ErrorMetrics};

let collector = MetricsCollector::new("my-pipeline");

// Record error
collector.record_error("Connection failed");

// Access error metrics
let metrics_handle = collector.metrics();
let errors = metrics_handle.metrics().errors();
let total_errors = errors.total_errors();
```

### Health Status

Track health status:

```rust
use streamweave_metrics::health::{HealthCheck, ComponentHealth, HealthStatus};

let mut health_check = HealthCheck::new("my-pipeline");

// Set component health
health_check.set_component_health(
    "database",
    ComponentHealth::healthy()
);

health_check.set_component_health(
    "cache",
    ComponentHealth::degraded("High latency")
);

// Check overall health
let status = health_check.status();
match status {
    HealthStatus::Healthy => println!("All systems operational"),
    HealthStatus::Degraded { reason } => println!("Degraded: {}", reason),
    HealthStatus::Unhealthy { reason } => println!("Unhealthy: {}", reason),
}
```

### Prometheus Integration

Export to Prometheus:

```rust
use streamweave_metrics::{MetricsCollector, PrometheusExporter};

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();

let exporter = PrometheusExporter::new("streamweave");
let prometheus_text = exporter.export(&metrics_handle.metrics());

// Serve via HTTP endpoint
// GET /metrics -> returns prometheus_text
```

### OpenTelemetry Integration

Export via OpenTelemetry:

```rust
use streamweave_metrics::{MetricsCollector, OpenTelemetryConfig};

let collector = MetricsCollector::new("my-pipeline");
let metrics_handle = collector.metrics();

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline");

let exporter = config.init_meter().await?;
// Export metrics via OTLP
```

## 🏗️ Architecture

Metrics collection flow:

```
Pipeline ──> MetricsCollector ──> PipelineMetrics ──> PrometheusExporter ──> Prometheus
Pipeline ──> MetricsCollector ──> PipelineMetrics ──> OpenTelemetryExporter ──> OTLP
Pipeline ──> HealthCheck ──> HealthStatus
```

**Metrics Flow:**
1. MetricsCollector collects metrics during execution
2. Metrics are aggregated into PipelineMetrics
3. Exporters convert metrics to external formats
4. Metrics are exported to monitoring systems

## 🔧 Configuration

### MetricsCollector

- **Pipeline Name**: Name for metrics identification
- **Collection Interval**: Interval for metrics collection

### HealthCheck

- **Pipeline Name**: Name for health identification
- **Component Health**: Individual component health status

### PrometheusExporter

- **Metric Prefix**: Prefix for metric names
- **Labels**: Additional labels for metrics

## 🔍 Error Handling

Metrics errors are handled gracefully:

```rust
use streamweave_metrics::MetricsCollector;

let collector = MetricsCollector::new("my-pipeline");
// Metrics collection continues even if individual operations fail
```

## ⚡ Performance Considerations

- **Low Overhead**: Minimal performance impact
- **Async Collection**: Metrics collected asynchronously
- **Efficient Aggregation**: Efficient metric aggregation

## 📝 Examples

For more examples, see:
- [Metrics Collection Example](https://github.com/Industrial/streamweave/tree/main/examples/metrics_collection)
- [Health Check Example](https://github.com/Industrial/streamweave/tree/main/examples/health_checks)
- [Prometheus Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/prometheus_integration)

## 🔗 Dependencies

`streamweave-metrics` depends on:

- `streamweave` - Core traits
- `opentelemetry` (optional) - OpenTelemetry integration
- `opentelemetry_sdk` (optional) - OpenTelemetry SDK
- `opentelemetry-otlp` (optional) - OTLP exporter
- `tokio` - Async runtime
- `serde` - Serialization support
- `chrono` - Time handling

## 🎯 Use Cases

Metrics integration is used for:

1. **Performance Monitoring**: Monitor pipeline performance
2. **Health Checks**: Track pipeline and component health
3. **Observability**: Comprehensive observability
4. **Alerting**: Alert on metrics thresholds
5. **Debugging**: Debug performance issues

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-metrics)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/metrics)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-integration-opentelemetry](../integrations/opentelemetry/README.md) - OpenTelemetry integration
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

