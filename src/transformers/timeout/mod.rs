//! Timeout transformer module.
//!
//! This module provides functionality for applying timeouts to stream processing,
//! ensuring that operations complete within a specified duration.

/// Input types for the timeout transformer.
pub mod input;
/// Output types for the timeout transformer.
pub mod output;
/// The timeout transformer implementation.
pub mod timeout_transformer;
/// Transformer trait implementation for timeout.
pub mod transformer;
