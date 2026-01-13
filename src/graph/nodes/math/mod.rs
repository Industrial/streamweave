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
//! - **MinNode**: Minimum of two values (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **MaxNode**: Maximum of two values (`configuration`, `in1`, `in2` → `out`, `error`)

pub mod abs_node;
pub mod common;
pub mod max_node;
pub mod min_node;

#[cfg(test)]
mod abs_node_test;
#[cfg(test)]
mod common_test;
#[cfg(test)]
mod max_node_test;
#[cfg(test)]
mod min_node_test;

pub use abs_node::AbsNode;
pub use max_node::MaxNode;
pub use min_node::MinNode;
