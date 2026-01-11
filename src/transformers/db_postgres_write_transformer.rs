//! # PostgreSQL Write Transformer
//!
//! Transformer that writes `DatabaseRow` data to PostgreSQL tables while passing the same
//! data through to the output stream. This enables persisting data to PostgreSQL while
//! continuing the main pipeline flow.
//!
//! ## Overview
//!
//! The PostgreSQL Write Transformer provides:
//!
//! - **Database Writing**: Writes `DatabaseRow` data to PostgreSQL tables
//! - **Pass-Through**: Outputs the same rows that were written
//! - **Batch Insertion**: Batches inserts for improved performance
//! - **Connection Pooling**: Manages PostgreSQL connection pool lifecycle
//! - **Error Handling**: Configurable error strategies for write failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<DatabaseRow>` - Database rows to write
//! - **Output**: `Message<DatabaseRow>` - The same rows (pass-through)
//!
//! ## Performance
//!
//! The transformer batches inserts for improved performance, flushing batches
//! based on size and time thresholds.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::DbPostgresWriteTransformer;
//! use streamweave::db::{DatabaseConsumerConfig, DatabaseType};
//!
//! let config = DatabaseConsumerConfig::default()
//!   .with_connection_url("postgresql://user:pass@localhost/dbname")
//!   .with_table_name("users");
//! let transformer = DbPostgresWriteTransformer::new(config);
//! // Input: [DatabaseRow, ...]
//! // Writes to PostgreSQL and outputs: [DatabaseRow, ...]
//! ```

use crate::db::{DatabaseConsumerConfig, DatabaseRow, DatabaseType};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{error, warn};

/// A transformer that writes DatabaseRow data to PostgreSQL while passing data through.
///
/// Each input DatabaseRow is written to the PostgreSQL table (with batching for performance),
/// and then the same row is output, enabling writing to database and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::DbPostgresWriteTransformer;
/// use streamweave::db::{DatabaseConsumerConfig, DatabaseType};
///
/// let config = DatabaseConsumerConfig::default()
///   .with_connection_url("postgresql://user:pass@localhost/dbname")
///   .with_table_name("users");
/// let transformer = DbPostgresWriteTransformer::new(config);
/// // Input: [DatabaseRow, ...]
/// // Writes to PostgreSQL and outputs: [DatabaseRow, ...]
/// ```
pub struct DbPostgresWriteTransformer {
  /// Database-specific configuration.
  db_config: DatabaseConsumerConfig,
  /// Transformer configuration.
  config: TransformerConfig<DatabaseRow>,
  /// Connection pool (shared across stream items).
  pool: Arc<Mutex<Option<sqlx::PgPool>>>,
  /// Buffer for batch insertion (shared across stream items).
  batch_buffer: Arc<Mutex<Vec<DatabaseRow>>>,
  /// Timestamp of last batch flush (shared across stream items).
  last_flush: Arc<Mutex<Instant>>,
}

impl DbPostgresWriteTransformer {
  /// Creates a new `DbPostgresWriteTransformer` with the given database configuration.
  ///
  /// # Arguments
  ///
  /// * `db_config` - Database consumer configuration.
  #[must_use]
  pub fn new(mut db_config: DatabaseConsumerConfig) -> Self {
    db_config.database_type = DatabaseType::Postgres;
    Self {
      db_config,
      config: TransformerConfig::default(),
      pool: Arc::new(Mutex::new(None)),
      batch_buffer: Arc::new(Mutex::new(Vec::new())),
      last_flush: Arc::new(Mutex::new(Instant::now())),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<DatabaseRow>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the database consumer configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseConsumerConfig {
    &self.db_config
  }
}

impl Clone for DbPostgresWriteTransformer {
  fn clone(&self) -> Self {
    Self {
      db_config: self.db_config.clone(),
      config: self.config.clone(),
      pool: Arc::clone(&self.pool),
      batch_buffer: Arc::clone(&self.batch_buffer),
      last_flush: Arc::clone(&self.last_flush),
    }
  }
}

impl Input for DbPostgresWriteTransformer {
  type Input = DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

impl Output for DbPostgresWriteTransformer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Transformer for DbPostgresWriteTransformer {
  type InputPorts = (DatabaseRow,);
  type OutputPorts = (DatabaseRow,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "db_postgres_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let db_config = self.db_config.clone();
    let pool = Arc::clone(&self.pool);
    let batch_buffer = Arc::clone(&self.batch_buffer);
    let last_flush = Arc::clone(&self.last_flush);

    Box::pin(async_stream::stream! {
      // Initialize pool on first item
      {
        let mut pool_guard = pool.lock().await;
        if pool_guard.is_none() {
          match create_pool(&db_config).await {
            Ok(p) => {
              *pool_guard = Some(p);
            }
            Err(e) => {
              error!(
                component = %component_name,
                error = %e,
                "Failed to create database connection pool"
              );
              return;
            }
          }
        }
      }

      let mut input_stream = input;
      while let Some(row) = input_stream.next().await {
        // Add to batch buffer
        {
          let mut buffer_guard = batch_buffer.lock().await;
          buffer_guard.push(row.clone());
        }

        // Check if we should flush
        let should_flush = {
          let buffer_guard = batch_buffer.lock().await;
          let flush_guard = last_flush.lock().await;
          buffer_guard.len() >= db_config.batch_size
            || flush_guard.elapsed() >= db_config.batch_timeout
        };

        if should_flush {
          let pool_guard = pool.lock().await;
          if let Some(ref p) = *pool_guard {
            let flush_result = {
              let mut buffer_guard = batch_buffer.lock().await;
              let mut flush_guard = last_flush.lock().await;
              let result = flush_batch(p, &db_config, &mut buffer_guard, &component_name).await;
              *flush_guard = Instant::now();
              result
            };

            match flush_result {
              Ok(_) => {
                // Success, continue
              }
              Err(e) => {
                let stream_error = StreamError::new(
                  e.to_string().into(),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: Some(row.clone()),
                    component_name: component_name.clone(),
                    component_type: std::any::type_name::<DbPostgresWriteTransformer>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<DbPostgresWriteTransformer>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy, &stream_error) {
                  ErrorAction::Stop => {
                    error!(
                      component = %component_name,
                      error = %stream_error,
                      "Stopping due to batch insert error"
                    );
                    return;
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Skipping batch due to error"
                    );
                    // Clear the batch buffer
                    let mut buffer_guard = batch_buffer.lock().await;
                    buffer_guard.clear();
                    continue;
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Retry not supported for batch inserts, skipping batch"
                    );
                    // Clear the batch buffer
                    let mut buffer_guard = batch_buffer.lock().await;
                    buffer_guard.clear();
                    continue;
                  }
                }
              }
            }
          }
        }

        // Pass through the row
        yield row;
      }

      // Final flush of remaining items
      {
        let pool_guard = pool.lock().await;
        if let Some(ref p) = *pool_guard {
          let mut buffer_guard = batch_buffer.lock().await;
          if !buffer_guard.is_empty()
            && let Err(e) = flush_batch(p, &db_config, &mut buffer_guard, &component_name).await {
              error!(
                component = %component_name,
                error = %e,
                batch_size = buffer_guard.len(),
                "Failed to insert final batch"
              );
            }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<DatabaseRow>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<DatabaseRow> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<DatabaseRow> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<DatabaseRow>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<DatabaseRow>) -> ErrorContext<DatabaseRow> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "db_postgres_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "db_postgres_write_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

async fn create_pool(
  config: &DatabaseConsumerConfig,
) -> Result<sqlx::PgPool, Box<dyn std::error::Error + Send + Sync + 'static>> {
  let pool_options = sqlx::postgres::PgPoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(config.idle_timeout)
    .max_lifetime(config.max_lifetime);

  let pool = pool_options.connect(&config.connection_url).await?;
  Ok(pool)
}

async fn flush_batch(
  pool: &sqlx::PgPool,
  config: &DatabaseConsumerConfig,
  batch: &mut Vec<DatabaseRow>,
  _component_name: &str,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
  if batch.is_empty() {
    return Ok(0);
  }

  let rows = std::mem::take(batch);

  let first_row = &rows[0];
  let columns: Vec<String> = if let Some(ref mapping) = config.column_mapping {
    first_row
      .fields
      .keys()
      .map(|k| mapping.get(k).cloned().unwrap_or_else(|| k.clone()))
      .collect()
  } else {
    first_row.columns()
  };

  if columns.is_empty() {
    return Err("No columns found in row".into());
  }

  let query = if let Some(ref custom_query) = config.insert_query {
    custom_query.clone()
  } else {
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();
    format!(
      "INSERT INTO {} ({}) VALUES ({})",
      config.table_name,
      columns.join(", "),
      placeholders.join(", ")
    )
  };

  if config.use_transactions {
    let mut tx = pool.begin().await?;
    let mut inserted = 0;

    for row in rows {
      let mut query_builder = sqlx::query(&query);

      for col in &columns {
        let field_name = if let Some(ref mapping) = config.column_mapping {
          mapping
            .iter()
            .find(|(_, v)| v.as_str() == col.as_str())
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| col.clone())
        } else {
          col.clone()
        };

        let value = row.fields.get(&field_name);
        query_builder = bind_value_postgres(query_builder, value)?;
      }

      query_builder.execute(&mut *tx).await?;
      inserted += 1;

      if let Some(tsize) = config.transaction_size
        && inserted % tsize == 0
      {
        tx.commit().await?;
        tx = pool.begin().await?;
      }
    }

    tx.commit().await?;
    Ok(inserted)
  } else {
    let mut inserted = 0;

    for row in rows {
      let mut query_builder = sqlx::query(&query);

      for col in &columns {
        let field_name = if let Some(ref mapping) = config.column_mapping {
          mapping
            .iter()
            .find(|(_, v)| v.as_str() == col.as_str())
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| col.clone())
        } else {
          col.clone()
        };

        let value = row.fields.get(&field_name);
        query_builder = bind_value_postgres(query_builder, value)?;
      }

      query_builder.execute(pool).await?;
      inserted += 1;
    }

    Ok(inserted)
  }
}

fn bind_value_postgres<'q>(
  query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
  value: Option<&serde_json::Value>,
) -> Result<
  sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
  Box<dyn std::error::Error + Send + Sync + 'static>,
> {
  let bound_query = match value {
    None | Some(serde_json::Value::Null) => query.bind(None::<Option<String>>),
    Some(serde_json::Value::Bool(b)) => query.bind(*b),
    Some(serde_json::Value::Number(n)) => {
      if let Some(i) = n.as_i64() {
        query.bind(i)
      } else if let Some(f) = n.as_f64() {
        query.bind(f)
      } else {
        return Err("Unsupported number type for PostgreSQL parameter".into());
      }
    }
    Some(serde_json::Value::String(s)) => query.bind(s.clone()),
    Some(serde_json::Value::Array(_) | serde_json::Value::Object(_)) => {
      let json_str = serde_json::to_string(value.unwrap())?;
      query.bind(json_str)
    }
  };
  Ok(bound_query)
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
