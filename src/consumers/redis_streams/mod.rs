//! Redis Streams consumer module.
//!
//! This module provides a consumer implementation for Redis Streams,
//! allowing StreamWeave pipelines to consume messages from Redis Streams.

/// Consumer trait implementation for Redis Streams.
pub mod consumer;
/// Input types for Redis Streams consumer.
pub mod input;
/// Output types for Redis Streams consumer.
pub mod output;
/// Redis Streams consumer implementation.
pub mod redis_streams_consumer;
