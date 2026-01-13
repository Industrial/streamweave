//! # String Operation Nodes
//!
//! This module provides nodes for string manipulation operations.
//!
//! ## Standard Port Pattern
//!
//! All string nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports (`in`, `in1`, `in2`, etc.)
//! - **Output Ports:** Data output ports (`out`, `true`, `false`, etc.), plus `error`

pub mod case_node;
pub mod common;
pub mod concat_node;
pub mod contains_node;
pub mod ends_with_node;
pub mod equal_node;
pub mod join_node;
pub mod length_node;
pub mod match_node;
pub mod replace_node;
pub mod slice_node;
pub mod split_node;
pub mod starts_with_node;

#[cfg(test)]
mod case_node_test;
#[cfg(test)]
mod concat_node_test;
#[cfg(test)]
mod contains_node_test;
#[cfg(test)]
mod ends_with_node_test;
#[cfg(test)]
mod equal_node_test;
#[cfg(test)]
mod join_node_test;
#[cfg(test)]
mod length_node_test;
#[cfg(test)]
mod match_node_test;
#[cfg(test)]
mod replace_node_test;
#[cfg(test)]
mod slice_node_test;
#[cfg(test)]
mod split_node_test;
#[cfg(test)]
mod starts_with_node_test;

pub use case_node::StringCaseNode;
pub use concat_node::StringConcatNode;
pub use contains_node::StringContainsNode;
pub use ends_with_node::StringEndsWithNode;
pub use equal_node::StringEqualNode;
pub use join_node::StringJoinNode;
pub use length_node::StringLengthNode;
pub use match_node::StringMatchNode;
pub use replace_node::StringReplaceNode;
pub use slice_node::StringSliceNode;
pub use split_node::StringSplitNode;
pub use starts_with_node::StringStartsWithNode;
