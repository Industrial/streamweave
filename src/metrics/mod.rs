//! # Metrics Module
//!
//! This module provides comprehensive observability for StreamWeave pipelines through
//! metrics collection, tracing, and monitoring integration.
//!
//! ## Core Metrics
//!
//! The metrics module tracks:
//! - **Throughput**: Items processed per second
//! - **Latency**: Processing time percentiles (p50, p95, p99)
//! - **Errors**: Error rates and types
//! - **Backpressure**: Indicators of pipeline congestion
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::metrics::{MetricsCollector, PipelineMetrics};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let collector = MetricsCollector::new("my-pipeline");
//! let metrics = collector.metrics();
//!
//! // Metrics are automatically collected during pipeline execution
//! // Access metrics at any time:
//! let throughput = metrics.throughput();
//! let latency_p95 = metrics.latency_p95();
//! # Ok(())
//! # }
//! ```

pub mod collector;
pub mod types;

pub use collector::{MetricsCollector, MetricsHandle};
pub use types::{
    BackpressureLevel, ErrorMetrics, LatencyMetrics, PipelineMetrics, ThroughputMetrics,
};

