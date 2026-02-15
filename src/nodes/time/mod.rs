//! # Time Operation Nodes
//!
//! This module provides nodes for time-based stream manipulation.
//!
//! ## Standard Port Pattern
//!
//! All time nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data input ports
//! - **Output Ports:** Data output ports (`out`, etc.), plus `error`
//!
//! ## Available Nodes
//!
//! - **DelayNode**: Delay items by duration (`configuration`, `in`, `duration` → `out`, `error`)
//! - **TimeoutNode**: Apply timeout to item reception (`configuration`, `in`, `timeout` → `out`, `error`)
//! - **TimerNode**: Generate periodic events (`configuration`, `interval` → `out`, `error`)
//! - **TimestampNode**: Add timestamp to items (`configuration`, `in` → `out`, `error`)
//! - **EventTimeExtractorNode**: Extract event time from payloads, add event_timestamp (configuration, in -> out, error)
//! - **CurrentTimeNode**: Generate current timestamp (`configuration`, `trigger` → `out`, `error`)
//! - **FormatTimeNode**: Format timestamps into strings (`configuration`, `in`, `format` → `out`, `error`)
//! - **ParseTimeNode**: Parse time strings into timestamps (`configuration`, `in`, `format` → `out`, `error`)

pub mod current_time_node;
#[cfg(test)]
pub mod current_time_node_test;
pub mod delay_node;
#[cfg(test)]
pub mod delay_node_test;
pub mod event_time_extractor_node;
pub mod format_time_node;
#[cfg(test)]
pub mod format_time_node_test;
pub mod parse_time_node;
#[cfg(test)]
pub mod parse_time_node_test;
pub mod timeout_node;
#[cfg(test)]
pub mod timeout_node_test;
pub mod timer_node;
#[cfg(test)]
pub mod timer_node_test;
pub mod timestamp_node;
#[cfg(test)]
pub mod timestamp_node_test;

pub use current_time_node::CurrentTimeNode;
pub use delay_node::DelayNode;
pub use event_time_extractor_node::{
  EventTimeExtractor, EventTimeExtractorConfig, EventTimeExtractorNode, event_time_extractor,
  event_time_from_map,
};
pub use format_time_node::FormatTimeNode;
pub use parse_time_node::ParseTimeNode;
pub use timeout_node::TimeoutNode;
pub use timer_node::TimerNode;
pub use timestamp_node::TimestampNode;
