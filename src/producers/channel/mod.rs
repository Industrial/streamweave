//! Channel producer module.
//!
//! This module provides the `ChannelProducer` which generates a stream from a `tokio::sync::mpsc::Receiver`.

/// The Channel producer implementation.
pub mod channel_producer;
/// Output type definitions for the Channel producer.
pub mod output;
/// Producer trait implementation for Channel.
pub mod producer;
