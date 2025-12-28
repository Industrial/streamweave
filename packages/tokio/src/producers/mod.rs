//! Tokio channel producer for StreamWeave
//!
//! This module provides a producer that reads items from tokio channels
//! and produces them into StreamWeave streams.

pub mod channel_producer;
pub mod output;
pub mod producer;

pub use channel_producer::*;
