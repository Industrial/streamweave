//! # Reduction Nodes
//!
//! This module provides nodes for reducing and grouping data from streams.
//!
//! ## Standard Port Pattern
//!
//! All reduction nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (required for function configuration), plus `in` (data stream)
//! - **Output Ports:** `out` (reduced/grouped result), plus `error`
//!
//! ## Behavior
//!
//! Reduction nodes process the entire input stream and apply configurable functions
//! to reduce, group, or aggregate data. They emit results when the stream ends.
//!
//! ## Available Nodes
//!
//! - **ReduceNode**: Apply reduction function with initial value (`configuration`, `in` → `out`, `error`)
//! - **ScanNode**: Apply reduction function with initial value, emitting intermediate results (`configuration`, `in` → `out`, `error`)
//! - **GroupByNode**: Group items by key (`configuration`, `in` → `out`, `error`)
//! - **AggregateNode**: Apply aggregator function (`configuration`, `in` → `out`, `error`)

pub mod aggregate_node;
pub mod aggregate_node_test;
pub mod group_by_node;
pub mod group_by_node_test;
pub mod reduce_node;
pub mod reduce_node_test;
pub mod scan_node;
/// Test module for scan node functionality.
pub mod scan_node_test;

pub use aggregate_node::{
  AggregateConfig, AggregateConfigWrapper, AggregateNode, AggregatorFunction, aggregate_config,
};
pub use group_by_node::{
  GroupByConfig, GroupByConfigWrapper, GroupByKeyFunction, GroupByNode, group_by_config,
};
pub use reduce_node::{
  ReduceConfig, ReduceConfigWrapper, ReduceFunction, ReduceNode, reduce_config,
};
pub use scan_node::{ScanConfig, ScanConfigWrapper, ScanFunction, ScanNode, scan_config};
