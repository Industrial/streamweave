//! Standard output (stdout) and standard error (stderr) consumers for StreamWeave
//!
//! This module provides consumers that write to stdout and stderr.

pub mod consumer;
pub mod input;
pub mod stderr_consumer;
pub mod stdout_consumer;

pub use stderr_consumer::*;
pub use stdout_consumer::*;
