//! Database query node for executing database queries in graphs.
//!
//! This module provides [`DatabaseQuery`], a graph node that executes database queries
//! from stream items. It takes SQL query strings as input and outputs database rows,
//! enabling dynamic query execution in graph-based pipelines. It wraps
//! [`DatabaseQueryTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`DatabaseQuery`] is useful for executing database queries in graph-based pipelines.
//! It supports multiple database types and allows dynamic query execution based on
//! stream data, making it ideal for data processing workflows that interact with
//! databases.
//!
//! # Key Concepts
//!
//! - **SQL Query Execution**: Executes SQL queries from stream items
//! - **Database Rows Output**: Outputs query results as `DatabaseRow` structures
//! - **Configurable Database**: Supports multiple database types through configuration
//! - **Transformer Wrapper**: Wraps `DatabaseQueryTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`DatabaseQuery`]**: Node that executes database queries from stream items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::DatabaseQuery;
//! use streamweave::db::DatabaseProducerConfig;
//!
//! // Create database configuration
//! let db_config = DatabaseProducerConfig {
//!     connection_string: "postgres://localhost/db".to_string(),
//!     // ... other configuration
//! };
//!
//! // Create a database query node
//! let db_query = DatabaseQuery::new(db_config);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::DatabaseQuery;
//! use streamweave::db::DatabaseProducerConfig;
//! use streamweave::ErrorStrategy;
//!
//! # let db_config = DatabaseProducerConfig {
//! #     connection_string: "postgres://localhost/db".to_string(),
//! # };
//! // Create a database query node with error handling
//! let db_query = DatabaseQuery::new(db_config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("db-query".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Database Integration**: Uses sqlx for type-safe, async database access
//! - **Dynamic Queries**: Supports dynamic SQL query execution from stream items
//! - **DatabaseRow Output**: Returns structured database rows for flexible processing
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`DatabaseQuery`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::db::DatabaseProducerConfig;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::DatabaseQueryTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that executes database queries from stream items.
///
/// This node wraps `DatabaseQueryTransformer` for use in graphs.
pub struct DatabaseQuery {
  /// The underlying query transformer
  transformer: DatabaseQueryTransformer,
}

impl DatabaseQuery {
  /// Creates a new `DatabaseQuery` node.
  pub fn new(db_config: DatabaseProducerConfig) -> Self {
    Self {
      transformer: DatabaseQueryTransformer::new(db_config),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Clone for DatabaseQuery {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for DatabaseQuery {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for DatabaseQuery {
  type Output = crate::db::DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = crate::db::DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DatabaseQuery {
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
