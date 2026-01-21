//! # String Operation Nodes
//!
//! This module provides nodes for string manipulation operations.
//!
//! ## Standard Port Pattern
//!
//! All string nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports (`in`, `in1`, `in2`, etc.)
//! - **Output Ports:** Data output ports (`out`, `true`, `false`, etc.), plus `error`

pub mod append_node;
pub mod capitalize_node;
pub mod case_node;
pub mod char_at_node;
pub mod common;
pub mod concat_node;
pub mod contains_node;
pub mod ends_with_node;
pub mod equal_node;
pub mod format_node;
pub mod index_of_node;
pub mod join_node;
pub mod last_index_of_node;
pub mod length_node;
pub mod lowercase_node;
pub mod match_node;
pub mod pad_node;
pub mod prepend_node;
pub mod replace_node;
pub mod reverse_node;
pub mod slice_node;
pub mod split_node;
pub mod starts_with_node;
pub mod trim_node;
pub mod uppercase_node;

#[cfg(test)]
mod append_node_test;
#[cfg(test)]
mod capitalize_node_test;
#[cfg(test)]
mod case_node_test;
#[cfg(test)]
mod char_at_node_test;
#[cfg(test)]
mod concat_node_test;
#[cfg(test)]
mod contains_node_test;
#[cfg(test)]
mod ends_with_node_test;
#[cfg(test)]
mod equal_node_test;
#[cfg(test)]
mod format_node_test;
#[cfg(test)]
mod index_of_node_test;
#[cfg(test)]
mod join_node_test;
#[cfg(test)]
mod last_index_of_node_test;
#[cfg(test)]
mod length_node_test;
#[cfg(test)]
mod lowercase_node_test;
#[cfg(test)]
mod match_node_test;
#[cfg(test)]
mod pad_node_test;
#[cfg(test)]
mod prepend_node_test;
#[cfg(test)]
mod replace_node_test;
#[cfg(test)]
mod reverse_node_test;
#[cfg(test)]
mod slice_node_test;
#[cfg(test)]
mod split_node_test;
#[cfg(test)]
mod starts_with_node_test;
#[cfg(test)]
mod trim_node_test;
#[cfg(test)]
mod uppercase_node_test;

pub use append_node::StringAppendNode;
pub use capitalize_node::StringCapitalizeNode;
pub use case_node::StringCaseNode;
pub use char_at_node::StringCharAtNode;
pub use concat_node::StringConcatNode;
pub use contains_node::StringContainsNode;
pub use ends_with_node::StringEndsWithNode;
pub use equal_node::StringEqualNode;
pub use format_node::StringFormatNode;
pub use index_of_node::StringIndexOfNode;
pub use join_node::StringJoinNode;
pub use last_index_of_node::StringLastIndexOfNode;
pub use length_node::StringLengthNode;
pub use lowercase_node::StringLowercaseNode;
pub use match_node::StringMatchNode;
pub use pad_node::StringPadNode;
pub use prepend_node::StringPrependNode;
pub use replace_node::StringReplaceNode;
pub use reverse_node::StringReverseNode;
pub use slice_node::StringSliceNode;
pub use split_node::StringSplitNode;
pub use starts_with_node::StringStartsWithNode;
pub use trim_node::StringTrimNode;
pub use uppercase_node::StringUppercaseNode;
