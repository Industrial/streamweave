//! # Math Function Nodes
//!
//! This module provides math function nodes for mathematical operations.
//!
//! ## Standard Port Pattern
//!
//! All math function nodes follow the standard port pattern for consistency:
//!
//! - **Input Ports**: `configuration` (currently unused, reserved for future use), plus data input ports
//! - **Output Ports**: Data output ports (`out` - result), plus `error`
//!
//! ## Available Nodes
//!
//! - **AbsNode**: Absolute value (`configuration`, `in` → `out`, `error`)

pub mod abs_node;
pub mod common;

#[cfg(test)]
mod abs_node_test;
#[cfg(test)]
mod common_test;

pub use abs_node::AbsNode;
