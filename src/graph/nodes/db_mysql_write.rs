//! MySQL write node for writing data to MySQL while passing data through.
//!
//! This module provides [`DbMysqlWrite`], a graph node that writes `DatabaseRow` data
//! to MySQL tables while passing the same data through to the output. It takes
//! `DatabaseRow` as input, writes it to a MySQL table, and outputs the same data,
//! enabling writing to database and continuing processing. It wraps
//! [`DbMysqlWriteTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`DbMysqlWrite`] is useful for writing intermediate results to MySQL tables while
//! continuing processing in graph-based pipelines. Unlike consumers, it passes data
//! through, making it ideal for checkpointing data at intermediate stages or
//! logging database writes.
//!
//! # Key Concepts
//!
//! - **Pass-Through Operation**: Writes data to MySQL while passing it through to output
//! - **DatabaseRow Input**: Takes `DatabaseRow` structures as input
//! - **Intermediate Results**: Enables writing intermediate results without
//!   interrupting the pipeline
//! - **Transformer Wrapper**: Wraps `DbMysqlWriteTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`DbMysqlWrite`]**: Node that writes data to MySQL while passing data through
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::DbMysqlWrite;
//! use streamweave::db::{DatabaseConsumerConfig, DatabaseRow};
//!
//! // Create database configuration
//! let config = DatabaseConsumerConfig::default()
//!     .with_connection_url("mysql://user:pass@localhost/dbname")
//!     .with_table_name("users");
//!
//! // Create a MySQL write node
//! let db_mysql_write = DbMysqlWrite::new(config);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::DbMysqlWrite;
//! use streamweave::db::{DatabaseConsumerConfig, DatabaseRow};
//! use streamweave::ErrorStrategy;
//!
//! # let config = DatabaseConsumerConfig::default()
//! #     .with_connection_url("mysql://user:pass@localhost/dbname")
//! #     .with_table_name("users");
//! // Create a MySQL write node with error handling
//! let db_mysql_write = DbMysqlWrite::new(config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("mysql-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Pass-Through Pattern**: Writes data while passing it through for
//!   intermediate result capture
//! - **MySQL Integration**: Uses sqlx for type-safe, async MySQL database access
//! - **DatabaseRow Support**: Works with `DatabaseRow` structures for structured
//!   database operations
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`DbMysqlWrite`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::db::{DatabaseConsumerConfig, DatabaseRow};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DbMysqlWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that writes DatabaseRow data to MySQL while passing data through.
///
/// This node wraps `DbMysqlWriteTransformer` for use in graphs. It takes DatabaseRow as input,
/// writes it to a MySQL table, and outputs the same data, enabling writing to database and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{DbMysqlWrite, TransformerNode};
/// use crate::db::{DatabaseConsumerConfig, DatabaseType};
///
/// let config = DatabaseConsumerConfig::default()
///   .with_connection_url("mysql://user:pass@localhost/dbname")
///   .with_table_name("users");
/// let db_mysql_write = DbMysqlWrite::new(config);
/// let node = TransformerNode::from_transformer(
///     "db_mysql_write".to_string(),
///     db_mysql_write,
/// );
/// ```
pub struct DbMysqlWrite {
  /// The underlying MySQL write transformer
  transformer: DbMysqlWriteTransformer,
}

impl DbMysqlWrite {
  /// Creates a new `DbMysqlWrite` node with the given database configuration.
  ///
  /// # Arguments
  ///
  /// * `db_config` - Database consumer configuration.
  pub fn new(db_config: DatabaseConsumerConfig) -> Self {
    Self {
      transformer: DbMysqlWriteTransformer::new(db_config),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<DatabaseRow>) -> Self {
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

  /// Returns the database consumer configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseConsumerConfig {
    self.transformer.db_config()
  }
}

impl Clone for DbMysqlWrite {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for DbMysqlWrite {
  type Input = DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

impl Output for DbMysqlWrite {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbMysqlWrite {
  type InputPorts = (DatabaseRow,);
  type OutputPorts = (DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<DatabaseRow>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<DatabaseRow> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<DatabaseRow> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<DatabaseRow>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<DatabaseRow>) -> ErrorContext<DatabaseRow> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
