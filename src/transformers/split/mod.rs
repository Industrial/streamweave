//! Split transformer module.
//!
//! This module provides functionality for splitting a stream of items into multiple streams
//! based on a predicate function.

/// Input types for the split transformer.
pub mod input;
/// Output types for the split transformer.
pub mod output;
/// The split transformer implementation.
pub mod split_transformer;
/// Transformer trait implementation for split.
pub mod transformer;
