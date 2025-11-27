//! Round-robin transformer module.
//!
//! This module provides the `RoundRobinTransformer` which distributes items across
//! multiple output streams in a round-robin fashion.

/// Input type definitions for the round-robin transformer.
pub mod input;
/// Output type definitions for the round-robin transformer.
pub mod output;
/// The round-robin transformer implementation.
pub mod round_robin_transformer;
/// Transformer trait implementation.
pub mod transformer;
