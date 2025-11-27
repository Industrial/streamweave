//! Channel consumer module.
//!
//! This module provides the `ChannelConsumer` which sends items to a `tokio::sync::mpsc::Sender`.

/// The Channel consumer implementation.
pub mod channel_consumer;
/// Consumer trait implementation for Channel.
pub mod consumer;
/// Input type definitions for the Channel consumer.
pub mod input;
