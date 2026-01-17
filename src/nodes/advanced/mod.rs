//! # Advanced Control Flow Nodes
//!
//! This module provides nodes for advanced control flow patterns including loops, error handling, and routing.
//!
//! ## Standard Port Pattern
//!
//! All advanced control nodes follow the standard port pattern:
//! - **Input Ports:** `configuration` (optional but should exist), plus data/control input ports
//! - **Output Ports:** Data output ports (`out`, `out_0`, `out_1`, etc.), plus `error`
//!
//! ## Available Nodes
//!
//! ### Advanced Loops
//! - **RepeatNode**: Repeat items N times (`configuration`, `in`, `count` → `out`, `error`)
//! - **BreakNode**: Break from loop on signal (`configuration`, `in`, `signal` → `out`, `error`)
//! - **ContinueNode**: Skip next item on signal (`configuration`, `in`, `signal` → `out`, `error`)
//!
//! ### Advanced Control
//! - **SwitchNode**: Multi-way switch routing (`configuration`, `in`, `value` → `out_0..out_n`, `default`, `error`)
//! - **TryCatchNode**: Try-catch error handling (`configuration`, `in`, `try`, `catch` → `out`, `error`)
//! - **RetryNode**: Retry on failure with exponential backoff (`configuration`, `in`, `max_retries` → `out`, `error`)

pub mod break_node;
pub mod break_node_test;
pub mod continue_node;
pub mod continue_node_test;
pub mod repeat_node;
pub mod repeat_node_test;
pub mod retry_node;
pub mod retry_node_test;
pub mod switch_node;
pub mod switch_node_test;
pub mod try_catch_node;
pub mod try_catch_node_test;

pub use break_node::BreakNode;
pub use continue_node::ContinueNode;
pub use repeat_node::RepeatNode;
pub use retry_node::{RetryConfig, RetryFunction, RetryNode, retry_config};
pub use switch_node::{SwitchConfig, SwitchFunction, SwitchNode, switch_config};
pub use try_catch_node::{
  CatchConfig, CatchFunction, TryCatchNode, TryConfig, TryFunction, catch_config, try_config,
};
