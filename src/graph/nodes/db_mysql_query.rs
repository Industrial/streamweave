//! MySQL query node for StreamWeave graphs
//!
//! Executes MySQL queries from stream items. Takes query strings (or query parameters) as input
//! and outputs query results, enabling dynamic MySQL queries in a pipeline.

use crate::db::DatabaseProducerConfig;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DbMysqlQueryTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that executes MySQL queries from stream items.
///
/// This node wraps `DbMysqlQueryTransformer` for use in graphs. It takes query strings
/// (or query parameters as JSON) as input and outputs `DatabaseRow` results, enabling
/// dynamic MySQL queries in a pipeline.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{DbMysqlQuery, TransformerNode};
/// use crate::db::{DatabaseProducerConfig, DatabaseType};
///
/// let db_config = DatabaseProducerConfig::default()
///   .with_connection_url("mysql://user:pass@localhost/db")
///   .with_database_type(DatabaseType::Mysql);
///
/// let db_mysql_query = DbMysqlQuery::new(db_config);
/// let node = TransformerNode::from_transformer(
///     "db_mysql_query".to_string(),
///     db_mysql_query,
/// );
/// ```
pub struct DbMysqlQuery {
  /// The underlying MySQL query transformer
  transformer: DbMysqlQueryTransformer,
}

impl DbMysqlQuery {
  /// Creates a new `DbMysqlQuery` node with the given database configuration.
  ///
  /// The database type in the config will be set to MySQL if not already set.
  pub fn new(db_config: DatabaseProducerConfig) -> Self {
    Self {
      transformer: DbMysqlQueryTransformer::new(db_config),
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

impl Clone for DbMysqlQuery {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for DbMysqlQuery {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for DbMysqlQuery {
  type Output = crate::db::DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = crate::db::DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbMysqlQuery {
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
