//! # Array Operation Nodes
//!
//! This module provides nodes for array manipulation operations.
//!
//! ## Standard Port Pattern
//!
//! All array nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports (`in`, `in1`, `in2`, etc.)
//! - **Output Ports:** Data output ports (`out`, etc.), plus `error`
//!
//! ## Available Nodes
//!
//! - **ArrayLengthNode**: Get array length (`configuration`, `in` → `out`, `error`)
//! - **ArrayIndexNode**: Get element at index (`configuration`, `in`, `index` → `out`, `error`)
//! - **ArraySliceNode**: Get array slice (`configuration`, `in`, `start`, `end` → `out`, `error`)
//! - **ArrayContainsNode**: Check if array contains value (`configuration`, `in`, `value` → `out`, `error`)
//! - **ArrayIndexOfNode**: Get index of value (`configuration`, `in`, `value` → `out`, `error`)
//! - **ArrayConcatNode**: Concatenate arrays (`configuration`, `in1`, `in2` → `out`, `error`)
//! - **ArrayReverseNode**: Reverse array (`configuration`, `in` → `out`, `error`)
//! - **ArraySortNode**: Sort array (`configuration`, `in` → `out`, `error`)
//! - **ArrayFilterNode**: Filter array elements (`configuration`, `in` → `out`, `error`)
//! - **ArrayMapNode**: Map array elements (`configuration`, `in` → `out`, `error`)
//! - **ArrayJoinNode**: Join array elements (`configuration`, `in`, `separator` → `out`, `error`)
//! - **ArraySplitNode**: Split array into chunks (`configuration`, `in`, `size` → `out`, `error`)
//! - **ArrayFlattenNode**: Flatten nested arrays (`configuration`, `in` → `out`, `error`)
//! - **ArrayUniqueNode**: Get unique elements (`configuration`, `in` → `out`, `error`)

pub mod common;
pub mod concat_node;
pub mod contains_node;
pub mod filter_node;
pub mod flatten_node;
pub mod index_node;
pub mod index_of_node;
pub mod join_node;
pub mod length_node;
pub mod map_node;
pub mod reverse_node;
pub mod slice_node;
pub mod sort_node;
pub mod split_node;
pub mod unique_node;

#[cfg(test)]
mod concat_node_test;
#[cfg(test)]
mod contains_node_test;
#[cfg(test)]
mod filter_node_test;
#[cfg(test)]
mod flatten_node_test;
#[cfg(test)]
mod index_node_test;
#[cfg(test)]
mod index_of_node_test;
#[cfg(test)]
mod join_node_test;
#[cfg(test)]
mod length_node_test;
#[cfg(test)]
mod map_node_test;
#[cfg(test)]
mod reverse_node_test;
#[cfg(test)]
mod slice_node_test;
#[cfg(test)]
mod sort_node_test;
#[cfg(test)]
mod split_node_test;
#[cfg(test)]
mod unique_node_test;

pub use concat_node::ArrayConcatNode;
pub use contains_node::ArrayContainsNode;
pub use filter_node::ArrayFilterNode;
pub use flatten_node::ArrayFlattenNode;
pub use index_node::ArrayIndexNode;
pub use index_of_node::ArrayIndexOfNode;
pub use join_node::ArrayJoinNode;
pub use length_node::ArrayLengthNode;
pub use map_node::ArrayMapNode;
pub use reverse_node::ArrayReverseNode;
pub use slice_node::ArraySliceNode;
pub use sort_node::ArraySortNode;
pub use split_node::ArraySplitNode;
pub use unique_node::ArrayUniqueNode;
