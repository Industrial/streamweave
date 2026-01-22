//! # Type Operation Nodes
//!
//! This module provides nodes for type checking and type conversion operations.
//!
//! ## Standard Port Pattern
//!
//! All type operation nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus `in` (value to check/convert)
//! - **Output Ports:** Data output ports (`out` - result), plus `error`
//!
//! ## Available Nodes
//!
//! ### Type Checking
//! - **TypeOfNode**: Get type name of value (`configuration`, `in` → `out`, `error`)
//! - **IsNumberNode**: Check if value is a number (`configuration`, `in` → `out`, `error`)
//! - **IsStringNode**: Check if value is a string (`configuration`, `in` → `out`, `error`)
//! - **IsBooleanNode**: Check if value is a boolean (`configuration`, `in` → `out`, `error`)
//! - **IsArrayNode**: Check if value is an array (`configuration`, `in` → `out`, `error`)
//! - **IsObjectNode**: Check if value is an object (`configuration`, `in` → `out`, `error`)
//! - **IsNullNode**: Check if value is null (`configuration`, `in` → `out`, `error`)
//!
//! ### Type Conversion
//! - **ToStringNode**: Convert value to string (`configuration`, `in` → `out`, `error`)
//! - **ToNumberNode**: Convert value to number (`configuration`, `in` → `out`, `error`)
//! - **ToFloatNode**: Convert value to float (`configuration`, `in` → `out`, `error`)
//! - **ToIntNode**: Convert value to integer (`configuration`, `in` → `out`, `error`)
//! - **ToBooleanNode**: Convert value to boolean (`configuration`, `in` → `out`, `error`)
//! - **ToArrayNode**: Convert value to array (`configuration`, `in` → `out`, `error`)

pub mod is_array_node;
#[cfg(test)]
pub mod is_array_node_test;
pub mod is_boolean_node;
#[cfg(test)]
pub mod is_boolean_node_test;
pub mod is_float_node;
#[cfg(test)]
pub mod is_float_node_test;
pub mod is_null_node;
#[cfg(test)]
pub mod is_null_node_test;
pub mod is_number_node;
#[cfg(test)]
pub mod is_number_node_test;
pub mod is_object_node;
#[cfg(test)]
pub mod is_object_node_test;
pub mod is_string_node;
#[cfg(test)]
pub mod is_string_node_test;
pub mod to_array_node;
#[cfg(test)]
pub mod to_array_node_test;
pub mod to_boolean_node;
#[cfg(test)]
pub mod to_boolean_node_test;
pub mod to_float_node;
#[cfg(test)]
pub mod to_float_node_test;
pub mod to_int_node;
#[cfg(test)]
pub mod to_int_node_test;
pub mod to_number_node;
#[cfg(test)]
pub mod to_number_node_test;
pub mod to_string_node;
#[cfg(test)]
pub mod to_string_node_test;
pub mod type_of_node;
#[cfg(test)]
pub mod type_of_node_test;

pub use is_array_node::IsArrayNode;
pub use is_boolean_node::IsBooleanNode;
pub use is_float_node::IsFloatNode;
pub use is_null_node::IsNullNode;
pub use is_number_node::IsNumberNode;
pub use is_object_node::IsObjectNode;
pub use is_string_node::IsStringNode;
pub use to_array_node::ToArrayNode;
pub use to_boolean_node::ToBooleanNode;
pub use to_float_node::ToFloatNode;
pub use to_int_node::ToIntNode;
pub use to_number_node::ToNumberNode;
pub use to_string_node::ToStringNode;
pub use type_of_node::TypeOfNode;
