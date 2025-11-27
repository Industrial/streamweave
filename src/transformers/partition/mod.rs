//! Partition transformer module.
//!
//! This module provides the `PartitionTransformer` which splits a stream into two streams
//! based on a predicate function.

/// Input type definitions for the partition transformer.
pub mod input;
/// Output type definitions for the partition transformer.
pub mod output;
/// The partition transformer implementation.
pub mod partition_transformer;
/// Transformer trait implementation.
pub mod transformer;
