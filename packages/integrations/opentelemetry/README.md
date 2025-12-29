# streamweave-integration-opentelemetry

[![Crates.io](https://img.shields.io/crates/v/streamweave-integration-opentelemetry.svg)](https://crates.io/crates/streamweave-integration-opentelemetry)
[![Documentation](https://docs.rs/streamweave-integration-opentelemetry/badge.svg)](https://docs.rs/streamweave-integration-opentelemetry)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**OpenTelemetry integration for StreamWeave**  
*Distributed tracing and metrics export for StreamWeave pipelines.*

The `streamweave-integration-opentelemetry` package provides OpenTelemetry integration for StreamWeave. It enables distributed tracing, metrics export via OTLP, and comprehensive observability for StreamWeave pipelines.

## ✨ Key Features

- **Distributed Tracing**: Automatic span creation for pipeline stages
- **Metrics Export**: Export metrics via OTLP (OpenTelemetry Protocol)
- **Context Propagation**: Trace context propagation through pipelines
- **OTLP Support**: Export to OpenTelemetry collectors
- **Tracing Integration**: Integration with `tracing` crate

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-integration-opentelemetry = { version = "0.3.0", features = ["opentelemetry"] }
```

## 🚀 Quick Start

### Basic Setup

```rust
use streamweave_integration_opentelemetry::OpenTelemetryConfig;

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline")
    .with_otlp_endpoint("http://localhost:4317");

let tracer = config.init_tracer().await?;
let meter = config.init_meter().await?;
```

### Tracing Pipelines

```rust
use streamweave_integration_opentelemetry::{OpenTelemetryConfig, PipelineTracer};
use streamweave_pipeline::PipelineBuilder;

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline");

let tracer = config.init_tracer().await?;
let pipeline_tracer = PipelineTracer::new(tracer);

let pipeline = PipelineBuilder::new()
    .producer(/* producer */)
    .transformer(/* transformer */)
    .consumer(/* consumer */);

// Pipeline execution is automatically traced
pipeline.run().await?;
```

### Metrics Export

```rust
use streamweave_integration_opentelemetry::{OpenTelemetryConfig, MetricsExporter};
use streamweave_metrics::PipelineMetrics;

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline");

let exporter = MetricsExporter::new("my-pipeline", config).await?;

let metrics = PipelineMetrics::new("my-pipeline");
exporter.export_metrics(&metrics).await;
```

## 📖 API Overview

### OpenTelemetryConfig

Configuration for OpenTelemetry integration:

```rust
pub struct OpenTelemetryConfig {
    pub service_name: String,
    pub otlp_endpoint: String,
    pub service_version: Option<String>,
    pub resource_attributes: Vec<KeyValue>,
}
```

**Key Methods:**
- `new()` - Create default configuration
- `with_service_name(name)` - Set service name
- `with_otlp_endpoint(endpoint)` - Set OTLP endpoint
- `with_service_version(version)` - Set service version
- `with_resource_attribute(key, value)` - Add resource attribute
- `init_tracer()` - Initialize tracer
- `init_meter()` - Initialize meter

### PipelineTracer

Tracer for pipeline operations:

```rust
pub struct PipelineTracer {
    // Internal state
}
```

**Key Methods:**
- `new(tracer)` - Create pipeline tracer
- `trace_pipeline()` - Trace pipeline execution

### MetricsExporter

Exports metrics via OTLP:

```rust
pub struct MetricsExporter {
    // Internal state
}
```

**Key Methods:**
- `new(pipeline_name, config)` - Create exporter
- `export_metrics(metrics)` - Export metrics

## 📚 Usage Examples

### Service Configuration

Configure service information:

```rust
use streamweave_integration_opentelemetry::OpenTelemetryConfig;

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline")
    .with_service_version("1.0.0")
    .with_otlp_endpoint("http://localhost:4317")
    .with_resource_attribute("environment", "production")
    .with_resource_attribute("team", "data-platform");
```

### Distributed Tracing

Enable distributed tracing:

```rust
use streamweave_integration_opentelemetry::{OpenTelemetryConfig, PipelineTracer};

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline");

let tracer = config.init_tracer().await?;
let pipeline_tracer = PipelineTracer::new(tracer);

// Traces are automatically created for pipeline stages
```

### Metrics Collection

Collect and export metrics:

```rust
use streamweave_integration_opentelemetry::{OpenTelemetryConfig, MetricsExporter};
use streamweave_metrics::PipelineMetrics;

let config = OpenTelemetryConfig::default()
    .with_service_name("my-pipeline");

let exporter = MetricsExporter::new("my-pipeline", config).await?;

// Export metrics periodically
let metrics = PipelineMetrics::new("my-pipeline");
exporter.export_metrics(&metrics).await;
```

### OTLP Endpoint Configuration

Configure OTLP endpoint:

```rust
use streamweave_integration_opentelemetry::OpenTelemetryConfig;

let config = OpenTelemetryConfig::default()
    .with_otlp_endpoint("http://otel-collector:4317");
```

## 🏗️ Architecture

OpenTelemetry integration flow:

```
Pipeline ──> PipelineTracer ──> OpenTelemetry Spans ──> OTLP ──> Collector
Pipeline ──> Metrics ──> MetricsExporter ──> OTLP ──> Collector
```

**OpenTelemetry Flow:**
1. Pipeline execution creates spans
2. Spans are exported via OTLP
3. Metrics are collected and exported
4. OpenTelemetry collector receives data
5. Data is forwarded to backends (Jaeger, Prometheus, etc.)

## 🔧 Configuration

### OpenTelemetryConfig

- **service_name**: Service name for traces and metrics
- **otlp_endpoint**: OTLP endpoint URL (default: http://localhost:4317)
- **service_version**: Optional service version
- **resource_attributes**: Additional resource attributes

## 🔍 Error Handling

OpenTelemetry errors are handled gracefully:

```rust
let config = OpenTelemetryConfig::default();
match config.init_tracer().await {
    Ok(tracer) => { /* use tracer */ }
    Err(e) => { /* handle error */ }
}
```

## ⚡ Performance Considerations

- **Batch Export**: Spans and metrics are batched for efficiency
- **Async Export**: Export operations are asynchronous
- **Resource Usage**: Minimal overhead on pipeline performance

## 📝 Examples

For more examples, see:
- [OpenTelemetry Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/opentelemetry_integration)
- [Observability Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-integration-opentelemetry` depends on:

- `streamweave` - Core traits
- `streamweave-metrics` (optional) - Metrics collection
- `opentelemetry` - OpenTelemetry SDK
- `opentelemetry_sdk` - OpenTelemetry SDK
- `opentelemetry-otlp` - OTLP exporter
- `tracing-opentelemetry` - Tracing integration
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

OpenTelemetry integration is used for:

1. **Distributed Tracing**: Trace requests across services
2. **Metrics Export**: Export metrics to monitoring systems
3. **Observability**: Comprehensive observability for pipelines
4. **Debugging**: Debug pipeline performance issues
5. **Monitoring**: Monitor pipeline health and performance

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-integration-opentelemetry)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/integrations/opentelemetry)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-metrics](../../metrics/README.md) - Metrics collection
- [streamweave-error](../../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

