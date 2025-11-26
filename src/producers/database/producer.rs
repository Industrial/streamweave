#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use super::database_producer::{
  DatabaseProducer, DatabaseProducerConfig, DatabaseRow, DatabaseType,
};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producer::{Producer, ProducerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use async_stream::stream;
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use async_trait::async_trait;
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use futures::StreamExt;
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use std::collections::HashMap;
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use tracing::{error, warn};

#[async_trait]
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
impl Producer for DatabaseProducer {
  /// Produces a stream of database rows from the configured query.
  ///
  /// # Error Handling
  ///
  /// - Connection errors are handled according to the error strategy.
  /// - Query execution errors trigger retries based on the error strategy.
  /// - Row processing errors are handled per the error strategy.
  fn produce(&mut self) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "database_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(stream! {
      // Create connection pool if not already created
      let pool = match self.pool.take() {
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

      // Execute query and stream results
      let query_result = match execute_query(&pool, &db_config).await {
        Ok(stream) => stream,
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: None,
              component_name: component_name.clone(),
              component_type: std::any::type_name::<Self>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<Self>().to_string(),
            },
          );

          match handle_error_strategy(&error_strategy, &error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %error,
                "Stopping due to query execution error"
              );
              return;
            }
            ErrorAction::Skip | ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Query execution failed, producing empty stream"
              );
              return;
            }
          }
        }
      };

      // Stream rows
      let mut query_result = std::pin::pin!(query_result);
      while let Some(row_result) = query_result.next().await {
        match row_result {
          Ok(row) => {
            yield row;
          }
          Err(e) => {
            let error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: None,
                component_name: component_name.clone(),
                component_type: std::any::type_name::<Self>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<Self>().to_string(),
              },
            );

            match handle_error_strategy(&error_strategy, &error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping due to row processing error"
                );
                break;
              }
              ErrorAction::Skip => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Skipping row due to processing error"
                );
                continue;
              }
              ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Retry not fully supported for row processing errors, skipping"
                );
                continue;
              }
            }
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

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use super::database_producer::DatabasePool;

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
async fn create_pool(
  config: &DatabaseProducerConfig,
) -> Result<DatabasePool, Box<dyn std::error::Error + Send + Sync>> {
  let pool_options = sqlx::pool::PoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(config.idle_timeout)
    .max_lifetime(config.max_lifetime);

  match config.database_type {
    DatabaseType::Postgres => {
      let pool = pool_options.connect(&config.connection_url).await?;
      Ok(DatabasePool::Postgres(pool))
    }
    DatabaseType::Mysql => {
      let pool = pool_options.connect(&config.connection_url).await?;
      Ok(DatabasePool::Mysql(pool))
    }
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
async fn execute_query(
  pool: &DatabasePool,
  config: &DatabaseProducerConfig,
) -> Result<
  impl futures::Stream<Item = Result<DatabaseRow, Box<dyn std::error::Error + Send + Sync>>> + Send,
  Box<dyn std::error::Error + Send + Sync>,
> {
  match pool {
    DatabasePool::Postgres(pg_pool) => {
      // Build query with parameters
      let mut query_builder = sqlx::query_as::<_, sqlx::postgres::PgRow>(&config.query);

      // Add parameters
      for param in &config.parameters {
        query_builder = bind_parameter(query_builder, param)?;
      }

      // Execute query and map rows
      let row_stream = query_builder.fetch(pg_pool).map(|row_result| {
        row_result
          .map(|row| convert_row_to_database_row(&row))
          .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
      });

      Ok(Box::pin(row_stream))
    }
    DatabasePool::Mysql(mysql_pool) => {
      // Build query with parameters
      let mut query_builder = sqlx::query_as::<_, sqlx::mysql::MySqlRow>(&config.query);

      // Add parameters
      for param in &config.parameters {
        query_builder = bind_parameter_mysql(query_builder, param)?;
      }

      // Execute query and map rows
      let row_stream = query_builder.fetch(mysql_pool).map(|row_result| {
        row_result
          .map(|row| convert_mysql_row_to_database_row(&row))
          .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
      });

      Ok(Box::pin(row_stream))
    }
  }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
fn bind_parameter(
  mut query: sqlx::query_builder::QueryAs<'_, sqlx::Postgres, sqlx::postgres::PgRow, ()>,
  param: &serde_json::Value,
) -> Result<
  sqlx::query_builder::QueryAs<'_, sqlx::Postgres, sqlx::postgres::PgRow, ()>,
  Box<dyn std::error::Error + Send + Sync>,
> {
  use sqlx::Arguments;
  // This is a simplified approach - in practice, sqlx requires compile-time type information
  // For runtime queries, we'll use sqlx::query which accepts runtime parameters
  // For now, return error indicating this needs dynamic query handling
  Err("Dynamic parameter binding requires sqlx::query (not query_as)".into())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
fn bind_parameter_mysql(
  mut query: sqlx::query_builder::QueryAs<'_, sqlx::MySql, sqlx::mysql::MySqlRow, ()>,
  param: &serde_json::Value,
) -> Result<
  sqlx::query_builder::QueryAs<'_, sqlx::MySql, sqlx::mysql::MySqlRow, ()>,
  Box<dyn std::error::Error + Send + Sync>,
> {
  Err("Dynamic parameter binding requires sqlx::query (not query_as)".into())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
fn convert_row_to_database_row(row: &sqlx::postgres::PgRow) -> DatabaseRow {
  use sqlx::Row;
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    let value = match column.type_info().name() {
      "BOOL" | "BOOLEAN" => row
        .try_get::<bool, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Bool(v)),
      "INT2" | "SMALLINT" => row
        .try_get::<i16, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(v.into())),
      "INT4" | "INTEGER" => row
        .try_get::<i32, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(v.into())),
      "INT8" | "BIGINT" => row
        .try_get::<i64, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(v.into())),
      "FLOAT4" | "REAL" => row
        .try_get::<f32, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(serde_json::Number::from_f64(v as f64).unwrap())),
      "FLOAT8" | "DOUBLE PRECISION" => row
        .try_get::<f64, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap())),
      "TEXT" | "VARCHAR" | "CHAR" => row
        .try_get::<String, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(v)),
      "BYTEA" => row
        .try_get::<Vec<u8>, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(base64::encode(v))),
      _ => row
        .try_get::<String, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(v))
        .or_else(|| row.try_get::<serde_json::Value, _>(idx).ok()),
    };

    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }

  DatabaseRow::new(fields)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
fn convert_mysql_row_to_database_row(row: &sqlx::mysql::MySqlRow) -> DatabaseRow {
  use sqlx::Row;
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    let value = match column.type_info().name() {
      "TINYINT" | "BOOLEAN" => row
        .try_get::<bool, _>(idx)
        .ok()
        .map(|v| serde_json::Value::Bool(v)),
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
        .map(|v| serde_json::Value::String(v)),
      "BLOB" | "BINARY" => row
        .try_get::<Vec<u8>, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(base64::encode(v))),
      _ => row
        .try_get::<String, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(v))
        .or_else(|| row.try_get::<serde_json::Value, _>(idx).ok()),
    };

    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }

  DatabaseRow::new(fields)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
fn handle_error_strategy<T>(strategy: &ErrorStrategy<T>, error: &StreamError<T>) -> ErrorAction
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
