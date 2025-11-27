//! Circuit breaker transformer module.
//!
//! This module provides functionality for implementing circuit breaker pattern
//! to prevent cascading failures in stream processing.

/// The circuit breaker transformer implementation.
pub mod circuit_breaker_transformer;
/// Input types for the circuit breaker transformer.
pub mod input;
/// Output types for the circuit breaker transformer.
pub mod output;
/// Transformer trait implementation for circuit breaker.
pub mod transformer;
