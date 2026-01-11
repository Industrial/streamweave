//! Database query transformer for executing database queries dynamically.
//!
//! This module provides [`DatabaseQueryTransformer`], a transformer that executes
//! database queries from stream items. It supports multiple database types (Postgres,
//! MySQL, SQLite) and provides flexible query input formats, making it useful for
//! graph composition and dynamic query execution.
//!
//! # Overview
//!
//! [`DatabaseQueryTransformer`] executes database queries for each input item. It
//! supports multiple database types via a unified interface, accepts queries as SQL
//! strings or JSON objects with parameters, and streams query results as `DatabaseRow`
//! items. This enables dynamic database querying within stream processing pipelines.
//!
//! # Key Concepts
//!
//! - **Multi-Database Support**: Supports Postgres, MySQL, and SQLite
//! - **Dynamic Queries**: Executes queries dynamically based on input items
//! - **Flexible Input**: Accepts SQL strings or JSON objects with query/parameters
//! - **Parameterized Queries**: Supports parameterized queries for security
//! - **Connection Pooling**: Manages database connection pool lifecycle
//! - **Result Streaming**: Streams query results as `DatabaseRow` items
//!
//! # Core Types
//!
//! - **[`DatabaseQueryTransformer`]**: Transformer that executes database queries
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::transformers::DatabaseQueryTransformer;
//! use streamweave::db::{DatabaseProducerConfig, DatabaseType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer with database configuration
//! let config = DatabaseProducerConfig::default()
//!     .with_connection_url("postgresql://user:pass@localhost/db")
//!     .with_database_type(DatabaseType::Postgres);
//!
//! let transformer = DatabaseQueryTransformer::new(config);
//!
//! // Input: ["SELECT * FROM users WHERE id = $1", ...]
//! // Output: [DatabaseRow, ...]
//! # Ok(())
//! # }
//! ```
//!
//! ## With JSON Input
//!
//! ```rust,no_run
//! use streamweave::transformers::DatabaseQueryTransformer;
//! use streamweave::db::{DatabaseProducerConfig, DatabaseType};
//!
//! // Input can be JSON with query and parameters
//! // Input: ["{\"query\": \"SELECT * FROM users WHERE id = $1\", \"parameters\": [1]}"]
//! ```
//!
//! # Design Decisions
//!
//! - **Multi-Database Support**: Uses `DatabaseType` enum for database type selection
//! - **Connection Pooling**: Creates connection pools based on database type
//! - **Flexible Input**: Supports both SQL strings and JSON objects with parameters
//! - **Parameterized Queries**: Uses parameterized queries for SQL injection prevention
//! - **Result Streaming**: Streams query results for efficient processing
//!
//! # Integration with StreamWeave
//!
//! [`DatabaseQueryTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline or graph. It supports the standard error handling
//! strategies and configuration options provided by [`TransformerConfig`].

use crate::db::{DatabaseProducerConfig, DatabaseRow, DatabaseType};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt, future::BoxFuture};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that executes database queries from stream items.
///
/// Input can be:
/// - A SQL query string (uses default connection and parameters)
/// - A JSON object with `query`, `parameters` fields
///
/// Output is a stream of `DatabaseRow` results from the query.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::DatabaseQueryTransformer;
/// use streamweave::db::{DatabaseProducerConfig, DatabaseType};
///
/// let config = DatabaseProducerConfig::default()
///   .with_connection_url("postgresql://user:pass@localhost/db")
///   .with_database_type(DatabaseType::Postgres);
///
/// let transformer = DatabaseQueryTransformer::new(config);
/// // Input: ["SELECT * FROM users WHERE id = $1", ...]
/// // Output: [DatabaseRow, ...]
/// ```
pub struct DatabaseQueryTransformer {
  /// Base database configuration
  db_config: DatabaseProducerConfig,
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl DatabaseQueryTransformer {
  /// Creates a new `DatabaseQueryTransformer` with the given database configuration.
  pub fn new(db_config: DatabaseProducerConfig) -> Self {
    Self {
      db_config,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Clone for DatabaseQueryTransformer {
  fn clone(&self) -> Self {
    Self {
      db_config: self.db_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for DatabaseQueryTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for DatabaseQueryTransformer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DatabaseQueryTransformer {
  type InputPorts = (String,);
  type OutputPorts = (DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "database_query_transformer".to_string());

    Box::pin(async_stream::stream! {
      // Create connection pool based on database type
      let pool_result = match db_config.database_type {
        DatabaseType::Postgres => {
          let pool = sqlx::PgPool::connect(&db_config.connection_url).await;
          pool.map(|p| Box::new(p) as Box<dyn DatabaseExecutor + Send + Sync>)
        }
        DatabaseType::Mysql => {
          let pool = sqlx::MySqlPool::connect(&db_config.connection_url).await;
          pool.map(|p| Box::new(p) as Box<dyn DatabaseExecutor + Send + Sync>)
        }
        DatabaseType::Sqlite => {
          let pool = sqlx::SqlitePool::connect(&db_config.connection_url).await;
          pool.map(|p| Box::new(p) as Box<dyn DatabaseExecutor + Send + Sync>)
        }
      };

      let pool = match pool_result {
        Ok(p) => p,
        Err(e) => {
          tracing::error!(
            component = %component_name,
            error = %e,
            "Failed to create database connection pool"
          );
          return;
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
        match pool.execute_query(&query, &params, db_config.database_type).await {
          Ok(rows) => {
            for row in rows {
              yield row;
            }
          }
          Err(e) => {
            tracing::error!(
              component = %component_name,
              error = %e,
              query = %query,
              "Failed to execute database query"
            );
            // Continue to next item on error
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
    match self.config.error_strategy {
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
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "database_query_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

trait DatabaseExecutor: Send + Sync {
  fn execute_query<'a>(
    &'a self,
    query: &'a str,
    params: &'a [Value],
    db_type: DatabaseType,
  ) -> BoxFuture<'a, Result<Vec<DatabaseRow>, Box<dyn std::error::Error + Send + Sync + 'static>>>;
}

impl DatabaseExecutor for sqlx::PgPool {
  fn execute_query<'a>(
    &'a self,
    query: &'a str,
    params: &'a [Value],
    _db_type: DatabaseType,
  ) -> BoxFuture<'a, Result<Vec<DatabaseRow>, Box<dyn std::error::Error + Send + Sync + 'static>>>
  {
    Box::pin(async move {
      let mut query_builder = sqlx::query(query);
      for param in params {
        query_builder = bind_value_postgres(query_builder, Some(param))?;
      }
      let rows = query_builder.fetch_all(self).await?;
      Ok(
        rows
          .into_iter()
          .map(|row| convert_postgres_row(&row))
          .collect(),
      )
    })
  }
}

impl DatabaseExecutor for sqlx::MySqlPool {
  fn execute_query<'a>(
    &'a self,
    query: &'a str,
    params: &'a [Value],
    _db_type: DatabaseType,
  ) -> BoxFuture<'a, Result<Vec<DatabaseRow>, Box<dyn std::error::Error + Send + Sync + 'static>>>
  {
    Box::pin(async move {
      let mut query_builder = sqlx::query(query);
      for param in params {
        query_builder = bind_value_mysql(query_builder, Some(param))?;
      }
      let rows = query_builder.fetch_all(self).await?;
      Ok(
        rows
          .into_iter()
          .map(|row| convert_mysql_row(&row))
          .collect(),
      )
    })
  }
}

impl DatabaseExecutor for sqlx::SqlitePool {
  fn execute_query<'a>(
    &'a self,
    query: &'a str,
    params: &'a [Value],
    _db_type: DatabaseType,
  ) -> BoxFuture<'a, Result<Vec<DatabaseRow>, Box<dyn std::error::Error + Send + Sync + 'static>>>
  {
    Box::pin(async move {
      let mut query_builder = sqlx::query(query);
      for param in params {
        query_builder = bind_value_sqlite(query_builder, Some(param))?;
      }
      let rows = query_builder.fetch_all(self).await?;
      Ok(
        rows
          .into_iter()
          .map(|row| convert_sqlite_row(&row))
          .collect(),
      )
    })
  }
}

// Reuse existing conversion functions from producers
fn convert_postgres_row(row: &sqlx::postgres::PgRow) -> DatabaseRow {
  use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
  use sqlx::{Column, Row};
  use std::collections::HashMap;

  let mut fields = HashMap::new();
  for (idx, column) in row.columns().iter().enumerate() {
    let value = match format!("{}", column.type_info()).as_str() {
      "BOOL" | "BOOLEAN" => row.try_get::<bool, _>(idx).ok().map(Value::Bool),
      "INT2" | "SMALLINT" => row
        .try_get::<i16, _>(idx)
        .ok()
        .map(|v| Value::Number(v.into())),
      "INT4" | "INTEGER" => row
        .try_get::<i32, _>(idx)
        .ok()
        .map(|v| Value::Number(v.into())),
      "INT8" | "BIGINT" => row
        .try_get::<i64, _>(idx)
        .ok()
        .map(|v| Value::Number(v.into())),
      "FLOAT4" | "REAL" => row
        .try_get::<f32, _>(idx)
        .ok()
        .map(|v| Value::Number(serde_json::Number::from_f64(v as f64).unwrap())),
      "FLOAT8" | "DOUBLE PRECISION" => row
        .try_get::<f64, _>(idx)
        .ok()
        .map(|v| Value::Number(serde_json::Number::from_f64(v).unwrap())),
      "TEXT" | "VARCHAR" | "CHAR" => row.try_get::<String, _>(idx).ok().map(Value::String),
      "BYTEA" => row
        .try_get::<Vec<u8>, _>(idx)
        .ok()
        .map(|v| Value::String(BASE64.encode(v))),
      _ => row
        .try_get::<String, _>(idx)
        .ok()
        .map(Value::String)
        .or_else(|| row.try_get::<Value, _>(idx).ok()),
    };
    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }
  DatabaseRow::new(fields)
}

fn convert_mysql_row(row: &sqlx::mysql::MySqlRow) -> DatabaseRow {
  use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
  use sqlx::{Column, Row};
  use std::collections::HashMap;

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

fn convert_sqlite_row(row: &sqlx::sqlite::SqliteRow) -> DatabaseRow {
  use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
  use sqlx::{Column, Row};
  use std::collections::HashMap;

  let mut fields = HashMap::new();
  for (idx, column) in row.columns().iter().enumerate() {
    let value = row
      .try_get::<i64, _>(idx)
      .ok()
      .map(|v| Value::Number(v.into()))
      .or_else(|| {
        row
          .try_get::<f64, _>(idx)
          .ok()
          .map(|v| Value::Number(serde_json::Number::from_f64(v).unwrap()))
      })
      .or_else(|| row.try_get::<String, _>(idx).ok().map(Value::String))
      .or_else(|| {
        row
          .try_get::<Vec<u8>, _>(idx)
          .ok()
          .map(|v| Value::String(BASE64.encode(v)))
      });
    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }
  DatabaseRow::new(fields)
}

fn bind_value_postgres<'q>(
  query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
  value: Option<&Value>,
) -> Result<
  sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
  Box<dyn std::error::Error + Send + Sync + 'static>,
> {
  match value {
    None | Some(Value::Null) => Ok(query.bind(None::<Option<String>>)),
    Some(Value::Bool(b)) => Ok(query.bind(*b)),
    Some(Value::Number(n)) => {
      if let Some(i) = n.as_i64() {
        Ok(query.bind(i))
      } else if let Some(f) = n.as_f64() {
        Ok(query.bind(f))
      } else {
        Err("Unsupported number type".into())
      }
    }
    Some(Value::String(s)) => Ok(query.bind(s.clone())),
    Some(Value::Array(_) | Value::Object(_)) => {
      let json_str = serde_json::to_string(value.unwrap())?;
      Ok(query.bind(json_str))
    }
  }
}

fn bind_value_mysql<'q>(
  query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  value: Option<&Value>,
) -> Result<
  sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  Box<dyn std::error::Error + Send + Sync + 'static>,
> {
  match value {
    None | Some(Value::Null) => Ok(query.bind(None::<Option<String>>)),
    Some(Value::Bool(b)) => Ok(query.bind(*b)),
    Some(Value::Number(n)) => {
      if let Some(i) = n.as_i64() {
        Ok(query.bind(i))
      } else if let Some(f) = n.as_f64() {
        Ok(query.bind(f))
      } else {
        Err("Unsupported number type".into())
      }
    }
    Some(Value::String(s)) => Ok(query.bind(s.clone())),
    Some(Value::Array(_) | Value::Object(_)) => {
      let json_str = serde_json::to_string(value.unwrap())?;
      Ok(query.bind(json_str))
    }
  }
}

fn bind_value_sqlite<'q>(
  query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  value: Option<&Value>,
) -> Result<
  sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  Box<dyn std::error::Error + Send + Sync + 'static>,
> {
  match value {
    None | Some(Value::Null) => Ok(query.bind(None::<Option<String>>)),
    Some(Value::Bool(b)) => Ok(query.bind(if *b { 1i64 } else { 0i64 })),
    Some(Value::Number(n)) => {
      if let Some(i) = n.as_i64() {
        Ok(query.bind(i))
      } else if let Some(f) = n.as_f64() {
        Ok(query.bind(f))
      } else {
        Err("Unsupported number type".into())
      }
    }
    Some(Value::String(s)) => Ok(query.bind(s.clone())),
    Some(Value::Array(_) | Value::Object(_)) => {
      let json_str = serde_json::to_string(value.unwrap())?;
      Ok(query.bind(json_str))
    }
  }
}
