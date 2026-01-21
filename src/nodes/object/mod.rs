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
pub mod object_delete_property_node;
pub mod object_entries_node;
pub mod object_has_property_node;
pub mod object_keys_node;
pub mod object_merge_node;
pub mod object_property_node;
pub mod object_set_property_node;
pub mod object_values_node;

#[cfg(test)]
mod object_delete_property_node_test;
#[cfg(test)]
mod object_entries_node_test;
#[cfg(test)]
mod object_has_property_node_test;
#[cfg(test)]
mod object_keys_node_test;
#[cfg(test)]
mod object_merge_node_test;
#[cfg(test)]
mod object_property_node_test;
#[cfg(test)]
mod object_set_property_node_test;
#[cfg(test)]
mod object_values_node_test;

pub use object_delete_property_node::ObjectDeletePropertyNode;
pub use object_entries_node::ObjectEntriesNode;
pub use object_has_property_node::ObjectHasPropertyNode;
pub use object_keys_node::ObjectKeysNode;
pub use object_merge_node::ObjectMergeNode;
pub use object_property_node::ObjectPropertyNode;
pub use object_set_property_node::ObjectSetPropertyNode;
pub use object_values_node::ObjectValuesNode;
