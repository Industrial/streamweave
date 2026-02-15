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
/// Test module for buffer node functionality.
pub mod buffer_node_test;
pub mod debounce_node;
#[cfg(test)]
/// Test module for debounce node functionality.
pub mod debounce_node_test;
pub mod distinct_node;
#[cfg(test)]
/// Test module for distinct node functionality.
pub mod distinct_node_test;
pub mod distinct_until_changed_node;
#[cfg(test)]
/// Test module for distinct until changed node functionality.
pub mod distinct_until_changed_node_test;
pub mod drop_node;
#[cfg(test)]
/// Test module for drop node functionality.
pub mod drop_node_test;
pub mod filter_map_node;
#[cfg(test)]
/// Test module for filter map node functionality.
pub mod filter_map_node_test;
pub mod first_node;
#[cfg(test)]
/// Test module for first node functionality.
pub mod first_node_test;
pub mod interleave_node;
#[cfg(test)]
/// Test module for interleave node functionality.
pub mod interleave_node_test;
pub mod last_node;
#[cfg(test)]
/// Test module for last node functionality.
pub mod last_node_test;
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
pub mod throttle_node;
#[cfg(test)]
/// Test module for throttle node functionality.
pub mod throttle_node_test;
pub mod watermark_injector_node;
#[cfg(test)]
pub mod watermark_injector_node_test;
pub mod session_event_time_window_node;
#[cfg(test)]
pub mod session_event_time_window_node_test;
pub mod sliding_event_time_window_node;
#[cfg(test)]
pub mod sliding_event_time_window_node_test;
pub mod tumbling_event_time_window_node;
#[cfg(test)]
pub mod tumbling_event_time_window_node_test;
pub mod tumbling_processing_time_window_node;
#[cfg(test)]
pub mod tumbling_processing_time_window_node_test;
pub mod window_node;
#[cfg(test)]
/// Test module for window node functionality.
pub mod window_node_test;
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
pub use first_node::FirstNode;
pub use interleave_node::InterleaveNode;
pub use last_node::LastNode;
pub use limit_node::LimitNode;
pub use merge_node::MergeNode;
pub use sample_node::SampleNode;
pub use skip_node::SkipNode;
pub use take_node::TakeNode;
pub use throttle_node::ThrottleNode;
pub use session_event_time_window_node::SessionEventTimeWindowNode;
pub use sliding_event_time_window_node::SlidingEventTimeWindowNode;
pub use tumbling_event_time_window_node::TumblingEventTimeWindowNode;
pub use watermark_injector_node::WatermarkInjectorNode;
pub use tumbling_processing_time_window_node::TumblingProcessingTimeWindowNode;
pub use window_node::WindowNode;
pub use zip_node::ZipNode;
