//! # String Operation Nodes
//!
//! This module provides nodes for string manipulation operations.
//!
//! ## Standard Port Pattern
//!
//! All string nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports (`in`, `in1`, `in2`, etc.)
//! - **Output Ports:** Data output ports (`out`, `true`, `false`, etc.), plus `error`

pub mod common;
pub mod concat_node;

#[cfg(test)]
mod concat_node_test;

pub use concat_node::StringConcatNode;
