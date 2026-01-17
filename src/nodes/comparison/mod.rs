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
//! - **NotEqualNode**: Inequality comparison (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **GreaterThanNode**: Greater than comparison (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **GreaterThanOrEqualNode**: Greater than or equal comparison (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **LessThanNode**: Less than comparison (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **LessThanOrEqualNode**: Less than or equal comparison (`configuration`, `in1`, `in2` → `out`, `error`)

pub mod common;
pub mod equal_node;
pub mod greater_than_node;
pub mod greater_than_or_equal_node;
pub mod less_than_node;
pub mod less_than_or_equal_node;
pub mod not_equal_node;

#[cfg(test)]
mod common_test;
#[cfg(test)]
mod equal_node_test;
#[cfg(test)]
mod greater_than_node_test;
#[cfg(test)]
mod greater_than_or_equal_node_test;
#[cfg(test)]
mod less_than_node_test;
#[cfg(test)]
mod less_than_or_equal_node_test;
#[cfg(test)]
mod not_equal_node_test;

pub use equal_node::EqualNode;
pub use greater_than_node::GreaterThanNode;
pub use greater_than_or_equal_node::GreaterThanOrEqualNode;
pub use less_than_node::LessThanNode;
pub use less_than_or_equal_node::LessThanOrEqualNode;
pub use not_equal_node::NotEqualNode;
