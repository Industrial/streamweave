//! CSV producer module.
//!
//! This module provides the `CsvProducer` which reads CSV files
//! and produces a stream of deserialized items.

/// The CSV producer implementation.
pub mod csv_producer;
/// Output type definitions for the CSV producer.
pub mod output;
/// Producer trait implementation for CSV.
pub mod producer;
