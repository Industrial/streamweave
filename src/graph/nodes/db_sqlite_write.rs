//! SQLite database write node for StreamWeave graphs
//!
//! Writes DatabaseRow data to SQLite while passing data through. Takes DatabaseRow as input,
//! writes to SQLite, and outputs the same data, enabling writing to database and continuing processing.

use crate::db::{DatabaseConsumerConfig, DatabaseRow};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DbSqliteWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that writes DatabaseRow data to SQLite while passing data through.
///
/// This node wraps `DbSqliteWriteTransformer` for use in graphs. It takes DatabaseRow as input,
/// writes it to a SQLite table, and outputs the same data, enabling writing to database and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{DbSqliteWrite, TransformerNode};
/// use crate::db::{DatabaseConsumerConfig, DatabaseType};
///
/// let config = DatabaseConsumerConfig::default()
///   .with_connection_url("sqlite:test.db")
///   .with_table_name("users");
/// let db_sqlite_write = DbSqliteWrite::new(config);
/// let node = TransformerNode::from_transformer(
///     "db_sqlite_write".to_string(),
///     db_sqlite_write,
/// );
/// ```
pub struct DbSqliteWrite {
  /// The underlying SQLite write transformer
  transformer: DbSqliteWriteTransformer,
}

impl DbSqliteWrite {
  /// Creates a new `DbSqliteWrite` node with the given database configuration.
  ///
  /// # Arguments
  ///
  /// * `db_config` - Database consumer configuration.
  pub fn new(db_config: DatabaseConsumerConfig) -> Self {
    Self {
      transformer: DbSqliteWriteTransformer::new(db_config),
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

impl Clone for DbSqliteWrite {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for DbSqliteWrite {
  type Input = DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

impl Output for DbSqliteWrite {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbSqliteWrite {
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
