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

pub mod add_node;
pub mod common;

#[cfg(test)]
mod add_node_test;
#[cfg(test)]
mod common_test;

pub use add_node::AddNode;
