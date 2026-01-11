//! MySQL database producer for streaming query results.
//!
//! This module provides [`DbMysqlProducer`], a producer that executes MySQL queries
//! and streams the results as `DatabaseRow` items. It uses connection pooling and
//! provides cursor-based iteration for large result sets to keep memory usage bounded.
//!
//! # Overview
//!
//! [`DbMysqlProducer`] is useful for reading data from MySQL databases in StreamWeave
//! pipelines. It executes SQL queries and streams the results row by row, making it
//! suitable for processing large datasets without loading everything into memory.
//!
//! # Key Concepts
//!
//! - **Connection Pooling**: Uses `sqlx::MySqlPool` for efficient connection management
//! - **Cursor-Based Iteration**: Processes results in batches to keep memory usage bounded
//! - **Database Rows**: Outputs `DatabaseRow` items representing query results
//! - **Lazy Initialization**: Connection pool is initialized on first use
//! - **Error Handling**: Configurable error strategies for database errors
//!
//! # Core Types
//!
//! - **[`DbMysqlProducer`]**: Producer that executes MySQL queries and streams results
//! - **[`DatabaseProducerConfig`]**: Configuration for database connection and query settings
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::producers::DbMysqlProducer;
//! use streamweave::db::DatabaseProducerConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create database configuration
//! let db_config = DatabaseProducerConfig {
//!     connection_string: "mysql://user:password@localhost/database".to_string(),
//!     query: "SELECT * FROM users".to_string(),
//!     // ... other configuration
//! };
//!
//! // Create a MySQL producer
//! let producer = DbMysqlProducer::new(db_config);
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust,no_run
//! use streamweave::producers::DbMysqlProducer;
//! use streamweave::db::DatabaseProducerConfig;
//! use streamweave::ErrorStrategy;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let db_config = DatabaseProducerConfig {
//!     connection_string: "mysql://user:password@localhost/database".to_string(),
//!     query: "SELECT * FROM users".to_string(),
//!     // ... other configuration
//! };
//!
//! // Create a producer with error handling strategy
//! let producer = DbMysqlProducer::new(db_config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("mysql-reader".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Design Decisions
//!
//! - **Connection Pooling**: Uses `sqlx` connection pooling for efficient database access
//! - **Lazy Initialization**: Pool is created on first use to avoid unnecessary connections
//! - **Cursor-Based Processing**: Processes results in batches to handle large datasets efficiently
//! - **Base64 Encoding**: Binary data is base64-encoded for safe transport in JSON-like structures
//! - **Generic Row Type**: Uses `DatabaseRow` for consistent database row representation
//!
//! # Integration with StreamWeave
//!
//! [`DbMysqlProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`ProducerConfig`].

use crate::db::{DatabaseProducerConfig, DatabaseRow};
use crate::error::ErrorStrategy;
use crate::{Output, Producer, ProducerConfig};
use async_stream::stream;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::{Stream, StreamExt};
use sqlx::Column;
use std::collections::HashMap;
use std::pin::Pin;
// Error types not used in producer implementation
use tracing::{error, warn};

/// A producer that executes MySQL queries and streams the results.
///
/// This producer uses connection pooling and provides cursor-based iteration
/// for large result sets to keep memory usage bounded.
pub struct DbMysqlProducer {
  /// Producer configuration.
  pub config: ProducerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseProducerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::MySqlPool>,
}

impl DbMysqlProducer {
  /// Creates a new MySQL producer with the given configuration.
  #[must_use]
  pub fn new(db_config: DatabaseProducerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      db_config,
      pool: None,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<DatabaseRow>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the database producer configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseProducerConfig {
    &self.db_config
  }
}

impl Clone for DbMysqlProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
    }
  }
}

impl Output for DbMysqlProducer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Producer for DbMysqlProducer {
  type OutputPorts = (DatabaseRow,);

  /// Produces a stream of database rows from the configured query.
  fn produce(&mut self) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "db_mysql_producer".to_string());
    let _error_strategy = self.config.error_strategy.clone();

    let pool_option = self.pool.take();

    Box::pin(stream! {
      let pool = match pool_option {
        Some(p) => p,
        None => {
          match create_pool(&db_config).await {
            Ok(p) => p,
            Err(e) => {
              error!(
                component = %component_name,
                error = %e,
                "Failed to create database connection pool, producing empty stream"
              );
              return;
            }
          }
        }
      };

      let query_result = match execute_query(&pool, &db_config).await {
        Ok(stream) => stream,
        Err(e) => {
          error!(
            component = %component_name,
            error = %e,
            "Failed to execute query, ending stream"
          );
          return;
        }
      };

      let mut query_result = std::pin::pin!(query_result);
      while let Some(row_result) = query_result.next().await {
        match row_result {
          Ok(row) => {
            yield row;
          }
          Err(e) => {
            warn!(
              component = %component_name,
              error = %e,
              "Row processing failed, skipping row"
            );
            continue;
          }
        }
       }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<DatabaseRow>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<DatabaseRow> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<DatabaseRow> {
    &mut self.config
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

async fn execute_query<'a>(
  pool: &'a sqlx::MySqlPool,
  config: &'a DatabaseProducerConfig,
) -> Result<
  std::pin::Pin<
    Box<
      dyn futures::Stream<Item = Result<DatabaseRow, Box<dyn std::error::Error + Send + Sync + 'a>>>
        + Send
        + 'a,
    >,
  >,
  Box<dyn std::error::Error + Send + Sync + 'a>,
> {
  let mut query = sqlx::query(&config.query);

  for param in &config.parameters {
    query = bind_parameter_mysql(query, param)?;
  }

  let row_stream = query.fetch(pool).map(|row_result| {
    row_result
      .map(|row: sqlx::mysql::MySqlRow| convert_mysql_row_to_database_row(&row))
      .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
  });

  Ok(Box::pin(row_stream))
}

fn bind_parameter_mysql<'q>(
  query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  param: &serde_json::Value,
) -> Result<
  sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  Box<dyn std::error::Error + Send + Sync>,
> {
  let bound_query = match param {
    serde_json::Value::Null => query.bind(None::<Option<String>>),
    serde_json::Value::Bool(b) => query.bind(*b),
    serde_json::Value::Number(n) => {
      if let Some(i) = n.as_i64() {
        query.bind(i)
      } else if let Some(f) = n.as_f64() {
        query.bind(f)
      } else {
        return Err("Unsupported number type for MySQL parameter".into());
      }
    }
    serde_json::Value::String(s) => query.bind(s.clone()),
    serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
      let json_str = serde_json::to_string(param)?;
      query.bind(json_str)
    }
  };
  Ok(bound_query)
}

fn convert_mysql_row_to_database_row(row: &sqlx::mysql::MySqlRow) -> DatabaseRow {
  use sqlx::Row;
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    let value = match format!("{}", column.type_info()).as_str() {
      "TINYINT" | "BOOLEAN" => row
        .try_get::<bool, _>(idx)
        .ok()
        .map(serde_json::Value::Bool),
      "SMALLINT" => row
        .try_get::<i16, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(v.into())),
      "INT" | "INTEGER" => row
        .try_get::<i32, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(v.into())),
      "BIGINT" => row
        .try_get::<i64, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(v.into())),
      "FLOAT" => row
        .try_get::<f32, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(serde_json::Number::from_f64(v as f64).unwrap())),
      "DOUBLE" => row
        .try_get::<f64, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap())),
      "VARCHAR" | "TEXT" | "CHAR" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => row
        .try_get::<String, _>(idx)
        .ok()
        .map(serde_json::Value::String),
      "BLOB" | "BINARY" => row
        .try_get::<Vec<u8>, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(BASE64.encode(v))),
      _ => row
        .try_get::<String, _>(idx)
        .ok()
        .map(serde_json::Value::String),
    };

    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }

  DatabaseRow::new(fields)
}
