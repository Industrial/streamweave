//! Database operation node for StreamWeave graphs
//!
//! Performs database operations (INSERT/UPDATE/DELETE) from stream items.

use crate::db::DatabaseConsumerConfig;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{DatabaseOperation, DatabaseOperationTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that performs database operations from stream items.
///
/// This node wraps `DatabaseOperationTransformer` for use in graphs.
pub struct DatabaseOperationNode {
  /// The underlying operation transformer
  transformer: DatabaseOperationTransformer,
}

impl DatabaseOperationNode {
  /// Creates a new `DatabaseOperationNode` node.
  pub fn new(db_config: DatabaseConsumerConfig, operation: DatabaseOperation) -> Self {
    Self {
      transformer: DatabaseOperationTransformer::new(db_config, operation),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<crate::db::DatabaseRow>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Clone for DatabaseOperationNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for DatabaseOperationNode {
  type Input = crate::db::DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = crate::db::DatabaseRow> + Send>>;
}

impl Output for DatabaseOperationNode {
  type Output = crate::db::DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = crate::db::DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DatabaseOperationNode {
  type InputPorts = (crate::db::DatabaseRow,);
  type OutputPorts = (crate::db::DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<crate::db::DatabaseRow>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<crate::db::DatabaseRow> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<crate::db::DatabaseRow> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<crate::db::DatabaseRow>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(
    &self,
    item: Option<crate::db::DatabaseRow>,
  ) -> ErrorContext<crate::db::DatabaseRow> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
