//! Standard input (stdin) producer for StreamWeave
//!
//! This module provides a producer that reads lines from standard input.

pub mod output;
pub mod producer;
pub mod stdin_producer;

pub use stdin_producer::*;
