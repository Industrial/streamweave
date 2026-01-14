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

pub mod delay_node;
pub mod delay_node_test;
pub mod timeout_node;
pub mod timeout_node_test;
pub mod timer_node;
pub mod timer_node_test;
pub mod timestamp_node;
pub mod timestamp_node_test;

pub use delay_node::DelayNode;
pub use timeout_node::TimeoutNode;
pub use timer_node::TimerNode;
pub use timestamp_node::TimestampNode;
