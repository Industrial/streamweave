//! Parquet consumer module.
//!
//! This module provides the `ParquetConsumer` which writes items to a Parquet file.

/// Consumer trait implementation for Parquet.
pub mod consumer;
/// Input type definitions for the Parquet consumer.
pub mod input;
/// The Parquet consumer implementation.
pub mod parquet_consumer;
