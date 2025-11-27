//! Kafka consumer module.
//!
//! This module provides a consumer implementation for Kafka,
//! allowing StreamWeave pipelines to consume messages from Kafka topics.

/// Consumer trait implementation for Kafka.
pub mod consumer;
/// Input types for Kafka consumer.
pub mod input;
/// Kafka consumer implementation.
pub mod kafka_consumer;
/// Output types for Kafka consumer.
pub mod output;
