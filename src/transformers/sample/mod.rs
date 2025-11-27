//! Sample transformer module.
//!
//! This module provides the `SampleTransformer` which randomly samples items from the stream
//! based on a given probability.

/// Input type definitions for the sample transformer.
pub mod input;
/// Output type definitions for the sample transformer.
pub mod output;
/// The sample transformer implementation.
pub mod sample_transformer;
/// Transformer trait implementation.
pub mod transformer;
