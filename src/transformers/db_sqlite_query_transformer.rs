//! SQLite query transformer for StreamWeave
//!
//! Executes SQLite queries from input items. Takes query strings (or query parameters) as input
//! and outputs query results, enabling dynamic SQLite queries in a pipeline.

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

/// A transformer that executes SQLite queries from input items.
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
/// use streamweave::transformers::DbSqliteQueryTransformer;
/// use streamweave::db::{DatabaseProducerConfig, DatabaseType};
///
/// let db_config = DatabaseProducerConfig::default()
///   .with_connection_url("sqlite://path/to/database.db")
///   .with_database_type(DatabaseType::Sqlite);
///
/// let transformer = DbSqliteQueryTransformer::new(db_config);
/// // Input: ["SELECT * FROM users WHERE id = ?", ...]
/// // Output: [DatabaseRow, ...]
/// ```
pub struct DbSqliteQueryTransformer {
  /// Base database configuration (connection URL, pool settings, etc.)
  db_config: DatabaseProducerConfig,
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl DbSqliteQueryTransformer {
  /// Creates a new `DbSqliteQueryTransformer` with the given database configuration.
  ///
  /// The database type in the config will be set to SQLite if not already set.
  #[must_use]
  pub fn new(mut db_config: DatabaseProducerConfig) -> Self {
    // Ensure database type is SQLite
    db_config.database_type = DatabaseType::Sqlite;
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

impl Clone for DbSqliteQueryTransformer {
  fn clone(&self) -> Self {
    Self {
      db_config: self.db_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for DbSqliteQueryTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for DbSqliteQueryTransformer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbSqliteQueryTransformer {
  type InputPorts = (String,);
  type OutputPorts = (DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "db_sqlite_query_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(async_stream::stream! {
      // Create SQLite connection pool
      let pool = match create_pool(&db_config).await {
        Ok(p) => p,
          Err(e) => {
            error!(
              component = %component_name,
              error = %e,
              "Failed to create SQLite connection pool"
            );
            let stream_error = StreamError::new(
              e,
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: None,
                component_name: component_name.clone(),
                component_type: std::any::type_name::<DbSqliteQueryTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<DbSqliteQueryTransformer>().to_string(),
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
                component_type: std::any::type_name::<DbSqliteQueryTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<DbSqliteQueryTransformer>().to_string(),
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
        .unwrap_or_else(|| "db_sqlite_query_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "db_sqlite_query_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

async fn create_pool(
  config: &DatabaseProducerConfig,
) -> Result<sqlx::SqlitePool, Box<dyn std::error::Error + Send + Sync>> {
  let pool_options = sqlx::sqlite::SqlitePoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(config.idle_timeout)
    .max_lifetime(config.max_lifetime);

  let pool = pool_options.connect(&config.connection_url).await?;
  Ok(pool)
}

async fn execute_query(
  pool: &sqlx::SqlitePool,
  query: &str,
  params: &[Value],
) -> Result<Vec<DatabaseRow>, Box<dyn std::error::Error + Send + Sync>> {
  let mut query_builder = sqlx::query(query);

  for param in params {
    query_builder = bind_parameter_sqlite(query_builder, param)?;
  }

  let rows = query_builder.fetch_all(pool).await?;
  Ok(
    rows
      .into_iter()
      .map(|row| convert_sqlite_row_to_database_row(&row))
      .collect(),
  )
}

fn bind_parameter_sqlite<'q>(
  query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  param: &Value,
) -> Result<
  sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  Box<dyn std::error::Error + Send + Sync>,
> {
  let bound_query = match param {
    Value::Null => query.bind(None::<Option<String>>),
    Value::Bool(b) => query.bind(if *b { 1i64 } else { 0i64 }), // SQLite uses integer for boolean
    Value::Number(n) => {
      if let Some(i) = n.as_i64() {
        query.bind(i)
      } else if let Some(f) = n.as_f64() {
        query.bind(f)
      } else {
        return Err("Unsupported number type for SQLite parameter".into());
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

fn convert_sqlite_row_to_database_row(row: &sqlx::sqlite::SqliteRow) -> DatabaseRow {
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    // SQLite TypeInfo doesn't have a name() method, so we try common types
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
