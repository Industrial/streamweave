//! Parquet producer module.
//!
//! This module provides the `ParquetProducer` which reads Apache Parquet files
//! and produces a stream of Arrow RecordBatches.

/// Output type definitions for the Parquet producer.
pub mod output;
/// The Parquet producer implementation.
pub mod parquet_producer;
/// Producer trait implementation for Parquet.
pub mod producer;
