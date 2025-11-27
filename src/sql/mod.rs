//! # SQL-Like Querying for StreamWeave
//!
//! This module provides a SQL-like query interface for stream processing,
//! allowing users to write intuitive SQL queries that are translated into
//! efficient StreamWeave pipelines.
//!
//! ## Overview
//!
//! The SQL query interface enables developers familiar with SQL to write
//! stream processing queries using familiar syntax. Queries are parsed,
//! optimized, and translated into StreamWeave pipelines.
//!
//! ## Supported SQL Constructs
//!
//! - **SELECT**: Project columns and compute expressions
//! - **WHERE**: Filter stream elements
//! - **GROUP BY**: Aggregate data by keys
//! - **WINDOW**: Define time-based or count-based windows
//! - **JOIN**: Join multiple streams
//! - **ORDER BY**: Sort results (with limitations for unbounded streams)
//! - **LIMIT**: Limit the number of results
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::sql::SqlQuery;
//!
//! let query = SqlQuery::parse(
//!     "SELECT user_id, COUNT(*) as event_count
//!      FROM events
//!      WHERE event_type = 'click'
//!      GROUP BY user_id
//!      WINDOW TUMBLING (SIZE 1 MINUTE)"
//! )?;
//!
//! let pipeline = query.to_pipeline()?;
//! let results = pipeline.run().await?;
//! ```
//!
//! ## Streaming Semantics
//!
//! Unlike traditional SQL which operates on bounded tables, SQL queries in
//! StreamWeave operate on unbounded streams. This affects the semantics:
//!
//! - **Aggregations** produce results continuously as windows close
//! - **ORDER BY** may be limited for unbounded streams
//! - **JOIN** requires windowing to bound the join state
//! - **Results** are emitted incrementally, not all at once

#[cfg(feature = "sql")]
pub mod ast;
#[cfg(feature = "sql")]
pub mod dialect;
#[cfg(feature = "sql")]
pub mod parser;
#[cfg(feature = "sql")]
pub mod translator;
#[cfg(feature = "sql")]
pub mod optimizer;

#[cfg(feature = "sql")]
pub use ast::{SqlAst, SqlQuery};
#[cfg(feature = "sql")]
pub use dialect::StreamSqlDialect;
#[cfg(feature = "sql")]
pub use parser::SqlParser;
#[cfg(feature = "sql")]
pub use translator::QueryTranslator;
#[cfg(feature = "sql")]
pub use optimizer::QueryOptimizer;

/// Convenience function to parse a SQL query
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::sql::parse_query;
///
/// let query = parse_query(
///     "SELECT user_id, COUNT(*) FROM events GROUP BY user_id"
/// )?;
/// ```
#[cfg(feature = "sql")]
pub fn parse_query(query: &str) -> Result<SqlQuery, crate::error::StreamError> {
  let parser = SqlParser::new();
  parser.parse(query)
}

