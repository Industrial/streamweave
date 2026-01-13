//! # Boolean Logic Nodes
//!
//! This module provides boolean logic operation nodes for processing boolean values
//! or values that can be evaluated as booleans.
//!
//! ## Available Nodes
//!
//! - **AndNode**: Logical AND operation
//! - **OrNode**: Logical OR operation
//! - **NotNode**: Logical NOT operation
//! - **XorNode**: Logical XOR (exclusive OR) operation
//! - **NandNode**: Logical NAND (NOT AND) operation
//! - **NorNode**: Logical NOR (NOT OR) operation

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
