//! # Aggregation Nodes
//!
//! This module provides nodes for aggregating data from streams.
//!
//! ## Standard Port Pattern
//!
//! All aggregation nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus `in` (data stream)
//! - **Output Ports:** `out` (single aggregated result), plus `error`
//!
//! ## Behavior
//!
//! Aggregation nodes process the entire input stream and emit a single result
//! when the stream ends. They maintain state throughout the stream processing.
//!
//! ## Available Nodes
//!
//! - **SumNode**: Sum numeric values (`configuration`, `in` → `out`, `error`)
//! - **CountNode**: Count items (`configuration`, `in` → `out`, `error`)
//! - **AverageNode**: Calculate average (`configuration`, `in` → `out`, `error`)
//! - **MinAggregateNode**: Find minimum value (`configuration`, `in` → `out`, `error`)
//! - **MaxAggregateNode**: Find maximum value (`configuration`, `in` → `out`, `error`)

pub mod average_node;
pub mod count_node;
pub mod max_aggregate_node;
pub mod min_aggregate_node;
pub mod sum_node;

#[cfg(test)]
mod average_node_test;
#[cfg(test)]
mod count_node_test;
#[cfg(test)]
mod max_aggregate_node_test;
#[cfg(test)]
mod min_aggregate_node_test;
#[cfg(test)]
mod sum_node_test;

pub use average_node::AverageNode;
pub use count_node::CountNode;
pub use max_aggregate_node::MaxAggregateNode;
pub use min_aggregate_node::MinAggregateNode;
pub use sum_node::SumNode;
