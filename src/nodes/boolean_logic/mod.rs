//! # Boolean Logic Nodes
//!
//! This module provides boolean logic operation nodes for processing boolean values
//! or values that can be evaluated as booleans.
//!
//! ## Standard Port Pattern
//!
//! All boolean logic nodes follow the standard port pattern for consistency:
//!
//! - **Input Ports**: `configuration` (currently unused, reserved for future use), plus data input ports
//! - **Output Ports**: Data output ports (`out`), plus `error`
//!
//! ## Available Nodes
//!
//! - **AndNode**: Logical AND operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **OrNode**: Logical OR operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **NotNode**: Logical NOT operation (`configuration`, `in` → `out`, `error`)
//! - **XorNode**: Logical XOR (exclusive OR) operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **NandNode**: Logical NAND (NOT AND) operation (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **NorNode**: Logical NOR (NOT OR) operation (`configuration`, `in1`, `in2` → `out`, `error`)
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use streamweave::nodes::boolean_logic::AndNode;
//! use streamweave::node::Node;
//!
//! // Create an AND node
//! let node = AndNode::new("and_gate".to_string());
//!
//! // All boolean logic nodes have the configuration port
//! assert!(node.has_input_port("configuration"));
//! assert!(node.has_input_port("in1"));  // or "in" for NotNode
//! assert!(node.has_output_port("out"));
//! assert!(node.has_output_port("error"));
//! ```

pub mod and_node;
pub mod common;
pub mod nand_node;
pub mod nor_node;
pub mod not_node;
pub mod or_node;
pub mod xor_node;

#[cfg(test)]
mod and_node_test;
#[cfg(test)]
mod nand_node_test;
#[cfg(test)]
mod nor_node_test;
#[cfg(test)]
mod not_node_test;
#[cfg(test)]
mod or_node_test;
#[cfg(test)]
mod xor_node_test;

pub use and_node::AndNode;
pub use nand_node::NandNode;
pub use nor_node::NorNode;
pub use not_node::NotNode;
pub use or_node::OrNode;
pub use xor_node::XorNode;
