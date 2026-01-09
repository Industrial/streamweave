//! Metrics collection for StreamWeave pipelines
//!
//! This module provides comprehensive metrics collection for StreamWeave pipelines.
//! It enables tracking throughput, latency, errors, backpressure, and health status,
//! with integration to Prometheus and OpenTelemetry.

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
