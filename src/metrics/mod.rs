//! # Metrics Collection for StreamWeave Pipelines
//!
//! Comprehensive metrics collection system for StreamWeave pipelines, enabling
//! observability, monitoring, and performance tracking. This module provides
//! metrics collection, health checks, and integration with Prometheus and
//! OpenTelemetry for production monitoring.
//!
//! ## Overview
//!
//! The Metrics module provides:
//!
//! - **Metrics Collection**: Track throughput, latency, errors, and backpressure
//! - **Health Checks**: Component and pipeline health status tracking
//! - **Prometheus Export**: Export metrics in Prometheus text format
//! - **OpenTelemetry Integration**: Distributed tracing and metrics via OTLP
//! - **Pipeline Observability**: Comprehensive observability for production pipelines
//!
//! ## Core Components
//!
//! - **MetricsCollector**: Main metrics collector for pipeline observability
//! - **PipelineMetrics**: Standard metric types (throughput, latency, errors, backpressure)
//! - **HealthCheck**: Health status tracking for components and pipelines
//! - **PrometheusExporter**: Prometheus metrics export
//! - **OpenTelemetryConfig**: OpenTelemetry integration configuration
//!
//! ## Metric Types
//!
//! - **Throughput**: Items processed, produced, and consumed per second
//! - **Latency**: P50, P95, P99 latency percentiles
//! - **Errors**: Error counts and error rates
//! - **Backpressure**: Backpressure indicators and queue depths
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::metrics::MetricsCollector;
//!
//! let collector = MetricsCollector::new("my-pipeline");
//! let metrics = collector.metrics();
//!
//! // Access metrics during/after pipeline execution
//! let throughput = metrics.throughput().items_per_second();
//! let latency = metrics.latency().latency_p95();
//! ```

pub mod collector;
pub mod health;
pub mod opentelemetry;
pub mod prometheus;
pub mod types;

pub use collector::*;
pub use health::*;
pub use opentelemetry::*;
pub use prometheus::*;
pub use types::*;
