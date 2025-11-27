//! Kafka producer module.
//!
//! This module provides the `KafkaProducer` which reads messages from Apache Kafka topics
//! and produces a stream of deserialized items.

/// Input type definitions for the Kafka producer.
pub mod input;
/// The Kafka producer implementation.
pub mod kafka_producer;
/// Output type definitions for the Kafka producer.
pub mod output;
/// Producer trait implementation for Kafka.
pub mod producer;
