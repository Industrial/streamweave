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
//! - **RoundNode**: Round to nearest integer (`configuration`, `in` → `out`, `error`)
//! - **FloorNode**: Floor to largest integer <= value (`configuration`, `in` → `out`, `error`)
//! - **CeilNode**: Ceil to smallest integer >= value (`configuration`, `in` → `out`, `error`)
//! - **SqrtNode**: Square root (`configuration`, `in` → `out`, `error`)
//! - **LogNode**: Logarithm (`configuration`, `in`, `base` → `out`, `error`)
//! - **ExpNode**: Exponential (e^x) (`configuration`, `in` → `out`, `error`)

pub mod abs_node;
pub mod ceil_node;
pub mod common;
pub mod exp_node;
pub mod floor_node;
pub mod log_node;
pub mod max_node;
pub mod min_node;
pub mod round_node;
pub mod sqrt_node;

#[cfg(test)]
mod abs_node_test;
#[cfg(test)]
mod ceil_node_test;
#[cfg(test)]
mod common_test;
#[cfg(test)]
mod exp_node_test;
#[cfg(test)]
mod floor_node_test;
#[cfg(test)]
mod log_node_test;
#[cfg(test)]
mod max_node_test;
#[cfg(test)]
mod min_node_test;
#[cfg(test)]
mod round_node_test;
#[cfg(test)]
mod sqrt_node_test;

pub use abs_node::AbsNode;
pub use ceil_node::CeilNode;
pub use exp_node::ExpNode;
pub use floor_node::FloorNode;
pub use log_node::LogNode;
pub use max_node::MaxNode;
pub use min_node::MinNode;
pub use round_node::RoundNode;
pub use sqrt_node::SqrtNode;
