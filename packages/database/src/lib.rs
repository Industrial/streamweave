//! Base types for StreamWeave database packages.
//!
//! This package provides common types used by database-specific implementations.
//! For actual database producers and consumers, use the specific packages:
//! - `streamweave-database-postgresql` for PostgreSQL
//! - `streamweave-database-mysql` for MySQL
//! - `streamweave-database-sqlite` for SQLite

pub mod types;

pub use types::{DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType};
