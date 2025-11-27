//! Database producer module.
//!
//! This module provides the `DatabaseProducer` which executes SQL queries against a database
//! and produces a stream of query results.

/// The database producer implementation.
pub mod database_producer;
/// Input type definitions for the database producer.
pub mod input;
/// Output type definitions for the database producer.
pub mod output;
/// Producer trait implementation for database.
pub mod producer;
