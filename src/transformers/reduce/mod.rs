//! Reduce transformer module.
//!
//! This module provides the `ReduceTransformer` which reduces a stream of items
//! into a single accumulated value.

/// Input type definitions for the reduce transformer.
pub mod input;
/// Output type definitions for the reduce transformer.
pub mod output;
/// The reduce transformer implementation.
pub mod reduce_transformer;
/// Transformer trait implementation.
pub mod transformer;
