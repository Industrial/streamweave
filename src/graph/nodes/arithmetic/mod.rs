//! # Arithmetic Nodes
//!
//! This module provides arithmetic operation nodes for performing mathematical operations
//! on numeric values.
//!
//! ## Standard Port Pattern
//!
//! All arithmetic nodes follow the standard port pattern for consistency:
//!
//! - **Input Ports**: `configuration` (currently unused, reserved for future use), plus data input ports
//! - **Output Ports**: Data output ports (`out`), plus `error`
//!
//! ## Available Nodes
//!
//! - **AddNode**: Addition operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **SubtractNode**: Subtraction operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **MultiplyNode**: Multiplication operation (`configuration`, `in1`, `in2` → `out`, `error`)

pub mod add_node;
pub mod common;
pub mod multiply_node;
pub mod subtract_node;

#[cfg(test)]
mod add_node_test;
#[cfg(test)]
mod common_test;
#[cfg(test)]
mod multiply_node_test;
#[cfg(test)]
mod subtract_node_test;

pub use add_node::AddNode;
pub use multiply_node::MultiplyNode;
pub use subtract_node::SubtractNode;
