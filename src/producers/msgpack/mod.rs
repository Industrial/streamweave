//! MessagePack producer module.
//!
//! This module provides the `MsgpackProducer` which reads MessagePack files
//! and produces a stream of deserialized items.

/// The MessagePack producer implementation.
pub mod msgpack_producer;
/// Output type definitions for the MessagePack producer.
pub mod output;
/// Producer trait implementation for MessagePack.
pub mod producer;
