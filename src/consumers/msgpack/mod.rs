//! MessagePack consumer module.
//!
//! This module provides the `MsgpackConsumer` which writes items to a MessagePack file.

/// Consumer trait implementation for MessagePack.
pub mod consumer;
/// Input type definitions for the MessagePack consumer.
pub mod input;
/// The MessagePack consumer implementation.
pub mod msgpack_consumer;
