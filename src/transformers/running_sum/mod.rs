//! Running sum stateful transformer.
//!
//! This module provides a stateful transformer that maintains a cumulative sum
//! of all items processed in the stream.

mod running_sum_transformer;
mod transformer;

pub use running_sum_transformer::RunningSumTransformer;
