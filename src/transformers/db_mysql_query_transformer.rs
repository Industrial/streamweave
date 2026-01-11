//! MySQL query transformer for executing MySQL database queries dynamically.
//!
//! This module provides [`DbMysqlQueryTransformer`], a transformer that executes
//! MySQL database queries for each input item, running queries dynamically based on
//! input data and producing query results as a stream.
//!
//! # Overview
//!
//! [`DbMysqlQueryTransformer`] executes MySQL queries for each input item. It supports
//! flexible input formats (SQL strings or JSON objects with query/parameters), uses
//! parameterized queries for security, and streams query results as `DatabaseRow` items.
//! This enables dynamic MySQL querying within stream processing pipelines.
//!
//! # Key Concepts
//!
//! - **Dynamic Query Execution**: Executes MySQL queries based on input items
//! - **Flexible Input**: Accepts SQL query strings or JSON objects with query/parameters
//! - **Parameterized Queries**: Supports parameterized queries (?) for security
//! - **Result Streaming**: Produces `DatabaseRow` results as a stream
//! - **Connection Pooling**: Manages MySQL connection pool lifecycle
//! - **Error Handling**: Configurable error strategies for query failures
//!
//! # Core Types
//!
//! - **[`DbMysqlQueryTransformer`]**: Transformer that executes MySQL queries
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::transformers::DbMysqlQueryTransformer;
//! use streamweave::db::{DatabaseProducerConfig, DatabaseType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let db_config = DatabaseProducerConfig::default()
//!     .with_connection_url("mysql://user:pass@localhost/db")
//!     .with_database_type(DatabaseType::Mysql);
//!
//! let transformer = DbMysqlQueryTransformer::new(db_config);
//! // Input: ["SELECT * FROM users WHERE id = ?", ...]
//! // Output: [DatabaseRow, ...]
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **MySQL-Specific**: Uses MySQL-specific connection pools and query syntax
//! - **Connection Pooling**: Creates MySQL connection pools for efficient connection reuse
//! - **Flexible Input**: Supports both SQL strings and JSON objects with parameters
//! - **Parameterized Queries**: Uses parameterized queries for SQL injection prevention
//! - **Result Streaming**: Streams query results for efficient processing
//!
//! # Integration with StreamWeave
//!
//! [`DbMysqlQueryTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline or graph. It supports the standard error handling
//! strategies and configuration options provided by [`TransformerConfig`].

use crate::db::{DatabaseProducerConfig, DatabaseRow, DatabaseType};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::{Stream, StreamExt};
use serde_json::Value;
use sqlx::{Column, Row};
use std::collections::HashMap;
use std::pin::Pin;
use tracing::error;

/// A transformer that executes MySQL queries from input items.
///
/// Input can be:
/// - A SQL query string (uses default connection and parameters from config)
/// - A JSON object with `query` and `parameters` fields
///
/// Output is a stream of `DatabaseRow` results from the query.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::DbMysqlQueryTransformer;
/// use streamweave::db::{DatabaseProducerConfig, DatabaseType};
///
/// let db_config = DatabaseProducerConfig::default()
///   .with_connection_url("mysql://user:pass@localhost/db")
///   .with_database_type(DatabaseType::Mysql);
///
/// let transformer = DbMysqlQueryTransformer::new(db_config);
/// // Input: ["SELECT * FROM users WHERE id = ?", ...]
/// // Output: [DatabaseRow, ...]
/// ```
pub struct DbMysqlQueryTransformer {
  /// Base database configuration (connection URL, pool settings, etc.)
  db_config: DatabaseProducerConfig,
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl DbMysqlQueryTransformer {
  /// Creates a new `DbMysqlQueryTransformer` with the given database configuration.
  ///
  /// The database type in the config will be set to MySQL if not already set.
  #[must_use]
  pub fn new(mut db_config: DatabaseProducerConfig) -> Self {
    // Ensure database type is MySQL
    db_config.database_type = DatabaseType::Mysql;
    Self {
      db_config,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the database configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseProducerConfig {
    &self.db_config
  }
}

impl Clone for DbMysqlQueryTransformer {
  fn clone(&self) -> Self {
    Self {
      db_config: self.db_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for DbMysqlQueryTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for DbMysqlQueryTransformer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbMysqlQueryTransformer {
  type InputPorts = (String,);
  type OutputPorts = (DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "db_mysql_query_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(async_stream::stream! {
      // Create MySQL connection pool
      let pool = match create_pool(&db_config).await {
        Ok(p) => p,
          Err(e) => {
            error!(
              component = %component_name,
              error = %e,
              "Failed to create MySQL connection pool"
            );
            let stream_error = StreamError::new(
              e,
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: None,
                component_name: component_name.clone(),
                component_type: std::any::type_name::<DbMysqlQueryTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<DbMysqlQueryTransformer>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy, &stream_error) {
              ErrorAction::Stop => {
                // Stop processing
                return;
              }
              ErrorAction::Skip => {
                // Skip and continue (but no pool, so return)
                return;
              }
              ErrorAction::Retry => {
                // Retry not directly supported for pool creation
                return;
              }
            }
          }
      };

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Parse input - could be query string or JSON object
        let (query, params) = if let Ok(json) = serde_json::from_str::<Value>(&item) {
          // JSON object with query and parameters
          let query_str = json
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| item.clone());
          let params = json
            .get("parameters")
            .and_then(|v| v.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default();
          (query_str, params)
        } else {
          // Simple query string - use default parameters from config
          (item, db_config.parameters.clone())
        };

        // Execute query
        match execute_query(&pool, &query, &params).await {
          Ok(rows) => {
            for row in rows {
              yield row;
            }
          }
          Err(e) => {
            let stream_error = StreamError::new(
              e,
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(query.clone()),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<DbMysqlQueryTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<DbMysqlQueryTransformer>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy, &stream_error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  query = %query,
                  error = %stream_error,
                  "Stopping due to query execution error"
                );
                return;
              }
              ErrorAction::Skip => {
                // Continue to next query
              }
              ErrorAction::Retry => {
                // Retry not directly supported for query execution
              }
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "db_mysql_query_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "db_mysql_query_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

async fn create_pool(
  config: &DatabaseProducerConfig,
) -> Result<sqlx::MySqlPool, Box<dyn std::error::Error + Send + Sync>> {
  let pool_options = sqlx::mysql::MySqlPoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(config.idle_timeout)
    .max_lifetime(config.max_lifetime);

  let pool = pool_options.connect(&config.connection_url).await?;
  Ok(pool)
}

async fn execute_query(
  pool: &sqlx::MySqlPool,
  query: &str,
  params: &[Value],
) -> Result<Vec<DatabaseRow>, Box<dyn std::error::Error + Send + Sync>> {
  let mut query_builder = sqlx::query(query);

  for param in params {
    query_builder = bind_parameter_mysql(query_builder, param)?;
  }

  let rows = query_builder.fetch_all(pool).await?;
  Ok(
    rows
      .into_iter()
      .map(|row| convert_mysql_row_to_database_row(&row))
      .collect(),
  )
}

fn bind_parameter_mysql<'q>(
  query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  param: &Value,
) -> Result<
  sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  Box<dyn std::error::Error + Send + Sync>,
> {
  let bound_query = match param {
    Value::Null => query.bind(None::<Option<String>>),
    Value::Bool(b) => query.bind(*b),
    Value::Number(n) => {
      if let Some(i) = n.as_i64() {
        query.bind(i)
      } else if let Some(f) = n.as_f64() {
        query.bind(f)
      } else {
        return Err("Unsupported number type for MySQL parameter".into());
      }
    }
    Value::String(s) => query.bind(s.clone()),
    Value::Array(_) | Value::Object(_) => {
      let json_str = serde_json::to_string(param)?;
      query.bind(json_str)
    }
  };
  Ok(bound_query)
}

fn convert_mysql_row_to_database_row(row: &sqlx::mysql::MySqlRow) -> DatabaseRow {
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    let value = match format!("{}", column.type_info()).as_str() {
      "TINYINT" | "BOOLEAN" => row.try_get::<bool, _>(idx).ok().map(Value::Bool),
      "SMALLINT" => row
        .try_get::<i16, _>(idx)
        .ok()
        .map(|v| Value::Number(v.into())),
      "INT" | "INTEGER" => row
        .try_get::<i32, _>(idx)
        .ok()
        .map(|v| Value::Number(v.into())),
      "BIGINT" => row
        .try_get::<i64, _>(idx)
        .ok()
        .map(|v| Value::Number(v.into())),
      "FLOAT" => row
        .try_get::<f32, _>(idx)
        .ok()
        .map(|v| Value::Number(serde_json::Number::from_f64(v as f64).unwrap())),
      "DOUBLE" => row
        .try_get::<f64, _>(idx)
        .ok()
        .map(|v| Value::Number(serde_json::Number::from_f64(v).unwrap())),
      "VARCHAR" | "TEXT" | "CHAR" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => {
        row.try_get::<String, _>(idx).ok().map(Value::String)
      }
      "BLOB" | "BINARY" => row
        .try_get::<Vec<u8>, _>(idx)
        .ok()
        .map(|v| Value::String(BASE64.encode(v))),
      _ => row.try_get::<String, _>(idx).ok().map(Value::String),
    };

    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }

  DatabaseRow::new(fields)
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy<T>(
  strategy: &ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}
