//! # Stream Control and Combination Nodes
//!
//! This module provides nodes for controlling stream flow and combining multiple streams.
//!
//! ## Standard Port Pattern
//!
//! All stream nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports
//! - **Output Ports:** Data output ports (`out`, etc.), plus `error`
//!
//! ## Available Nodes
//!
//! ### Stream Control
//! - **TakeNode**: Take first N items (`configuration`, `in`, `count` → `out`, `error`)
//! - **SkipNode**: Skip first N items (`configuration`, `in`, `count` → `out`, `error`)
//! - **LimitNode**: Limit total items (`configuration`, `in`, `limit` → `out`, `error`)
//! - **DropNode**: Discard all items (`configuration`, `in` → `out`, `error`)
//! - **BufferNode**: Buffer items into batches (`configuration`, `in`, `size` → `out`, `error`)
//! - **SampleNode**: Sample items at rate (`configuration`, `in`, `rate` → `out`, `error`)
//!
//! ### Stream Combination
//! - **ZipNode**: Combine multiple streams (`configuration`, `in1`, `in2`, ... → `out`, `error`)
//! - **InterleaveNode**: Interleave multiple streams (`configuration`, `in1`, `in2`, ... → `out`, `error`)
//! - **MergeNode**: Merge multiple streams (`configuration`, `in1`, `in2`, ... → `out`, `error`)

pub mod buffer_node;
#[cfg(test)]
pub mod buffer_node_test;
pub mod debounce_node;
#[cfg(test)]
pub mod debounce_node_test;
pub mod distinct_node;
#[cfg(test)]
pub mod distinct_node_test;
pub mod distinct_until_changed_node;
#[cfg(test)]
pub mod distinct_until_changed_node_test;
pub mod drop_node;
#[cfg(test)]
pub mod drop_node_test;
pub mod filter_map_node;
#[cfg(test)]
pub mod filter_map_node_test;
pub mod interleave_node;
#[cfg(test)]
pub mod interleave_node_test;
pub mod limit_node;
#[cfg(test)]
pub mod limit_node_test;
pub mod merge_node;
#[cfg(test)]
pub mod merge_node_test;
pub mod sample_node;
#[cfg(test)]
pub mod sample_node_test;
pub mod skip_node;
#[cfg(test)]
pub mod skip_node_test;
pub mod take_node;
#[cfg(test)]
pub mod take_node_test;
pub mod zip_node;
#[cfg(test)]
pub mod zip_node_test;

pub use buffer_node::BufferNode;
pub use debounce_node::DebounceNode;
pub use distinct_node::DistinctNode;
pub use distinct_until_changed_node::DistinctUntilChangedNode;
pub use drop_node::DropNode;
pub use filter_map_node::{
  FilterMapConfig, FilterMapConfigWrapper, FilterMapFunction, FilterMapNode, filter_map_config,
};
pub use interleave_node::InterleaveNode;
pub use limit_node::LimitNode;
pub use merge_node::MergeNode;
pub use sample_node::SampleNode;
pub use skip_node::SkipNode;
pub use take_node::TakeNode;
pub use zip_node::ZipNode;
