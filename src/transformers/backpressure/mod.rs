//! Backpressure transformer module.
//!
//! This module provides functionality for managing backpressure in streams,
//! controlling the flow of data to prevent overwhelming downstream components.

/// The backpressure transformer implementation.
pub mod backpressure_transformer;
/// Input types for the backpressure transformer.
pub mod input;
/// Output types for the backpressure transformer.
pub mod output;
/// Transformer trait implementation for backpressure.
pub mod transformer;
