//! # Object Operation Nodes
//!
//! This module provides nodes for object (HashMap) manipulation operations.
//!
//! ## Standard Port Pattern
//!
//! All object nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports (`in`, `key`, etc.)
//! - **Output Ports:** Data output ports (`out`, etc.), plus `error`
//!
//! ## Available Nodes
//!
//! - **ObjectKeysNode**: Get object keys (`configuration`, `in` → `out`, `error`)
//! - **ObjectValuesNode**: Get object values (`configuration`, `in` → `out`, `error`)
//! - **ObjectEntriesNode**: Get object entries (`configuration`, `in` → `out`, `error`)
//! - **ObjectPropertyNode**: Get property value (`configuration`, `in`, `key` → `out`, `error`)
//! - **ObjectHasPropertyNode**: Check if property exists (`configuration`, `in`, `key` → `out`, `error`)
//! - **ObjectMergeNode**: Merge objects (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **ObjectSetPropertyNode**: Set property value (`configuration`, `in`, `key`, `value` → `out`, `error`)
//! - **ObjectDeletePropertyNode**: Delete property (`configuration`, `in`, `key` → `out`, `error`)

pub mod common;
pub mod delete_property_node;
pub mod entries_node;
pub mod has_property_node;
pub mod keys_node;
pub mod merge_node;
pub mod property_node;
pub mod set_property_node;
pub mod values_node;

#[cfg(test)]
mod delete_property_node_test;
#[cfg(test)]
mod entries_node_test;
#[cfg(test)]
mod has_property_node_test;
#[cfg(test)]
mod keys_node_test;
#[cfg(test)]
mod merge_node_test;
#[cfg(test)]
mod property_node_test;
#[cfg(test)]
mod set_property_node_test;
#[cfg(test)]
mod values_node_test;

pub use delete_property_node::ObjectDeletePropertyNode;
pub use entries_node::ObjectEntriesNode;
pub use has_property_node::ObjectHasPropertyNode;
pub use keys_node::ObjectKeysNode;
pub use merge_node::ObjectMergeNode;
pub use property_node::ObjectPropertyNode;
pub use set_property_node::ObjectSetPropertyNode;
pub use values_node::ObjectValuesNode;
