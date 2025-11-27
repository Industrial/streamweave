//! Join transformer module.
//!
//! This module provides functionality for joining two streams based on matching keys,
//! similar to SQL joins, with configurable time windows.

/// Input types for the join transformer.
pub mod input;
/// The join transformer implementation.
pub mod join_transformer;
/// Output types for the join transformer.
pub mod output;
/// Transformer trait implementation for join.
pub mod transformer;
