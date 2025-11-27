//! Rate limit transformer module.
//!
//! This module provides functionality for rate limiting items in a stream,
//! ensuring that only a specified number of items are processed within a time window.

/// Input types for the rate limit transformer.
pub mod input;
/// Output types for the rate limit transformer.
pub mod output;
/// The rate limit transformer implementation.
pub mod rate_limit_transformer;
/// Transformer trait implementation for rate limit.
pub mod transformer;
