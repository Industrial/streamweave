//! Flat map transformer module.
//!
//! This module provides the `FlatMapTransformer` which applies a function to each item
//! and flattens the resulting vectors into a single stream.

/// The flat map transformer implementation.
pub mod flat_map_transformer;
/// Input type definitions for the flat map transformer.
pub mod input;
/// Output type definitions for the flat map transformer.
pub mod output;
/// Transformer trait implementation.
pub mod transformer;
