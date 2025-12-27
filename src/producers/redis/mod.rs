//! Redis Streams producer module.
//!
//! This module provides the `RedisProducer` which reads messages from Redis Streams
//! and produces a stream of deserialized items.

/// Input type definitions for the Redis Streams producer.
pub mod input;
/// Output type definitions for the Redis Streams producer.
pub mod output;
/// Producer trait implementation for Redis Streams.
pub mod producer;
/// The Redis Streams producer implementation.
pub mod redis_producer;
