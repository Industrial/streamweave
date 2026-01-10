//! MySQL database write node for StreamWeave graphs
//!
//! Writes DatabaseRow data to MySQL while passing data through. Takes DatabaseRow as input,
//! writes to MySQL, and outputs the same data, enabling writing to database and continuing processing.

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
