//! Built-in consumer implementations.
//!
//! This module provides various consumer implementations for different use cases,
//! including in-memory collections, file I/O, and external system integrations.

/// Array consumer for collecting items into arrays.
pub mod array;
/// Channel consumer for sending items through channels.
pub mod channel;
/// HashMap consumer for collecting items into hash maps.
pub mod hash_map;
/// HashSet consumer for collecting items into hash sets.
pub mod hash_set;
/// String consumer for collecting items into strings.
pub mod string;
/// Vec consumer for collecting items into vectors.
pub mod vec;

// Native-only consumers (require OS features and tokio runtime features)
#[cfg(feature = "native")]
/// Command consumer for executing shell commands.
pub mod command;
#[cfg(feature = "native")]
/// Console consumer for printing items to the console.
pub mod console;
#[cfg(feature = "native")]
/// File consumer for writing items to files.
pub mod file;

// File format consumers (native only, require file I/O)
#[cfg(feature = "file-formats")]
/// CSV consumer for writing items in CSV format.
pub mod csv;
#[cfg(feature = "file-formats")]
/// JSONL consumer for writing items in JSON Lines format.
pub mod jsonl;
#[cfg(feature = "file-formats")]
/// MessagePack consumer for writing items in MessagePack format.
pub mod msgpack;
#[cfg(feature = "file-formats")]
/// Parquet consumer for writing items in Parquet format.
pub mod parquet;

// Kafka integration (native only, requires librdkafka and cmake)
#[cfg(feature = "kafka")]
/// Kafka consumer for producing items to Kafka topics.
pub mod kafka;

// Redis Streams integration (native only, requires Redis)
#[cfg(feature = "redis")]
/// Redis Streams consumer for producing items to Redis streams.
pub mod redis;
