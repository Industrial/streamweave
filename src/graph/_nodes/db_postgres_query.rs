//! PostgreSQL query node for executing PostgreSQL queries in graphs.
//!
//! This module provides [`DbPostgresQuery`], a graph node that executes PostgreSQL queries
//! from stream items. It takes query strings (or query parameters as JSON) as input
//! and outputs `DatabaseRow` results, enabling dynamic PostgreSQL queries in graph-based
//! pipelines. It wraps [`DbPostgresQueryTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`DbPostgresQuery`] is useful for executing PostgreSQL queries in graph-based pipelines.
//! It supports dynamic query execution based on stream data, making it ideal for
//! data processing workflows that interact with PostgreSQL databases.
//!
//! # Key Concepts
//!
//! - **PostgreSQL Query Execution**: Executes SQL queries against PostgreSQL databases
//! - **Dynamic Queries**: Supports query strings or query parameters from stream items
//! - **Database Rows Output**: Outputs query results as `DatabaseRow` structures
//! - **Transformer Wrapper**: Wraps `DbPostgresQueryTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`DbPostgresQuery`]**: Node that executes PostgreSQL queries from stream items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::DbPostgresQuery;
//! use streamweave::db::{DatabaseProducerConfig, DatabaseType};
//!
//! // Create database configuration
//! let db_config = DatabaseProducerConfig::default()
//!     .with_connection_url("postgresql://user:pass@localhost/db")
//!     .with_database_type(DatabaseType::Postgres);
//!
//! // Create a PostgreSQL query node
//! let db_postgres_query = DbPostgresQuery::new(db_config);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::DbPostgresQuery;
//! use streamweave::db::{DatabaseProducerConfig, DatabaseType};
//! use streamweave::ErrorStrategy;
//!
//! # let db_config = DatabaseProducerConfig::default()
//! #     .with_connection_url("postgresql://user:pass@localhost/db")
//! #     .with_database_type(DatabaseType::Postgres);
//! // Create a PostgreSQL query node with error handling
//! let db_postgres_query = DbPostgresQuery::new(db_config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("postgres-query".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **PostgreSQL Integration**: Uses sqlx for type-safe, async PostgreSQL database access
//! - **Dynamic Queries**: Supports dynamic SQL query execution from stream items
//! - **DatabaseRow Output**: Returns structured database rows for flexible processing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`DbPostgresQuery`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::db::DatabaseProducerConfig;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DbPostgresQueryTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that executes PostgreSQL queries from stream items.
///
/// This node wraps `DbPostgresQueryTransformer` for use in graphs. It takes query strings
/// (or query parameters as JSON) as input and outputs `DatabaseRow` results, enabling
/// dynamic PostgreSQL queries in a pipeline.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{DbPostgresQuery, TransformerNode};
/// use crate::db::{DatabaseProducerConfig, DatabaseType};
///
/// let db_config = DatabaseProducerConfig::default()
///   .with_connection_url("postgresql://user:pass@localhost/db")
///   .with_database_type(DatabaseType::Postgres);
///
/// let db_postgres_query = DbPostgresQuery::new(db_config);
/// let node = TransformerNode::from_transformer(
///     "db_postgres_query".to_string(),
///     db_postgres_query,
/// );
/// ```
pub struct DbPostgresQuery {
  /// The underlying PostgreSQL query transformer
  transformer: DbPostgresQueryTransformer,
}

impl DbPostgresQuery {
  /// Creates a new `DbPostgresQuery` node with the given database configuration.
  ///
  /// The database type in the config will be set to PostgreSQL if not already set.
  pub fn new(db_config: DatabaseProducerConfig) -> Self {
    Self {
      transformer: DbPostgresQueryTransformer::new(db_config),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }

  /// Returns the database configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseProducerConfig {
    self.transformer.db_config()
  }
}

impl Clone for DbPostgresQuery {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for DbPostgresQuery {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for DbPostgresQuery {
  type Output = crate::db::DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = crate::db::DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbPostgresQuery {
  type InputPorts = (String,);
  type OutputPorts = (crate::db::DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
