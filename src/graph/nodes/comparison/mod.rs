//! # Comparison Nodes
//!
//! This module provides comparison operation nodes for comparing values.
//!
//! ## Standard Port Pattern
//!
//! All comparison nodes follow the standard port pattern for consistency:
//!
//! - **Input Ports**: `configuration` (currently unused, reserved for future use), plus data input ports
//! - **Output Ports**: Data output ports (`out` - boolean result), plus `error`
//!
//! ## Available Nodes
//!
//! - **EqualNode**: Equality comparison (`configuration`, `in1`, `in2` → `out`, `error`)

pub mod common;
pub mod equal_node;

#[cfg(test)]
mod common_test;
#[cfg(test)]
mod equal_node_test;

pub use equal_node::EqualNode;
