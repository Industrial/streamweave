//! PostgreSQL database consumer for writing stream data to PostgreSQL tables.
//!
//! This module provides [`DbPostgresConsumer`], a consumer that writes [`DatabaseRow`]
//! items to PostgreSQL database tables. It supports batch insertion for performance,
//! connection pooling, and transaction management through sqlx.
//!
//! # Overview
//!
//! [`DbPostgresConsumer`] is useful for persisting stream data to PostgreSQL databases.
//! It uses sqlx for type-safe database access, connection pooling for efficiency,
//! and batch insertion for high-throughput scenarios. The consumer handles
//! connection management, error handling, and transaction boundaries automatically.
//!
//! # Key Concepts
//!
//! - **DatabaseRow Input**: Items must be [`DatabaseRow`] structures representing
//!   database table rows
//! - **Batch Insertion**: Groups multiple rows into batches for efficient insertion
//! - **Connection Pooling**: Uses sqlx connection pools for efficient connection
//!   management
//! - **Transaction Management**: Supports configurable transaction handling
//!
//! # Core Types
//!
//! - **[`DbPostgresConsumer`]**: Consumer that writes rows to PostgreSQL tables
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::DbPostgresConsumer;
//! use streamweave::db::{DatabaseConsumerConfig, DatabaseRow};
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create database configuration
//! let db_config = DatabaseConsumerConfig {
//!     connection_string: "postgres://user:pass@localhost/db".to_string(),
//!     table_name: "records".to_string(),
//!     batch_size: 100,
//!     // ... other configuration
//! };
//!
//! // Create a consumer
//! let mut consumer = DbPostgresConsumer::new(db_config);
//!
//! // Create a stream of database rows
//! let stream = stream::iter(vec![
//!     DatabaseRow::new(vec!["value1".into(), "value2".into()]),
//!     // ... more rows
//! ]);
//!
//! // Consume the stream (rows inserted into PostgreSQL)
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::consumers::DbPostgresConsumer;
//! use streamweave::db::DatabaseConsumerConfig;
//! use streamweave::ErrorStrategy;
//!
//! let db_config = DatabaseConsumerConfig { /* ... */ };
//! let consumer = DbPostgresConsumer::new(db_config)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("postgres-writer".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **sqlx Integration**: Uses sqlx for type-safe, async database access
//! - **Batch Insertion**: Groups rows into batches for improved performance
//! - **Connection Pooling**: Reuses connections efficiently through sqlx pools
//! - **Lazy Pool Initialization**: Connection pool is created on first use
//!
//! # Integration with StreamWeave
//!
//! [`DbPostgresConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::db::{DatabaseConsumerConfig, DatabaseRow};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::time::Instant;
use tracing::{error, warn};

/// A consumer that writes database rows to a PostgreSQL table.
///
/// This consumer supports batch insertion for performance and transaction management.
pub struct DbPostgresConsumer {
  /// Consumer configuration.
  pub config: ConsumerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseConsumerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::PgPool>,
  /// Buffer for batch insertion.
  pub(crate) batch_buffer: Vec<DatabaseRow>,
  /// Timestamp of last batch flush.
  pub(crate) last_flush: Instant,
}

impl DbPostgresConsumer {
  /// Creates a new PostgreSQL consumer with the given configuration.
  #[must_use]
  pub fn new(db_config: DatabaseConsumerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      db_config,
      pool: None,
      batch_buffer: Vec::new(),
      last_flush: Instant::now(),
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<DatabaseRow>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Returns the database consumer configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseConsumerConfig {
    &self.db_config
  }
}

impl Clone for DbPostgresConsumer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
      batch_buffer: Vec::new(),
      last_flush: Instant::now(),
    }
  }
}

impl Input for DbPostgresConsumer {
  type Input = DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Consumer for DbPostgresConsumer {
  type InputPorts = (DatabaseRow,);

  /// Consumes a stream of database rows and inserts them into the configured table.
  async fn consume(&mut self, mut stream: Self::InputStream) {
    let component_name = if self.config.name.is_empty() {
      "db_postgres_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let db_config = self.db_config.clone();

    // Create connection pool if not already created
    let pool = match self.pool.take() {
      Some(p) => p,
      None => match create_pool(&db_config).await {
        Ok(p) => p,
        Err(e) => {
          error!(
            component = %component_name,
            error = %e,
            "Failed to create database connection pool, dropping all items"
          );
          return;
        }
      },
    };

    let mut total_inserted = 0usize;
    let mut total_errors = 0usize;

    while let Some(row) = stream.next().await {
      // Add to batch buffer
      self.batch_buffer.push(row);

      // Check if we should flush the batch
      let should_flush = self.batch_buffer.len() >= db_config.batch_size
        || self.last_flush.elapsed() >= db_config.batch_timeout;

      if should_flush {
        match flush_batch(&pool, &db_config, &mut self.batch_buffer, &component_name).await {
          Ok(count) => {
            total_inserted += count;
            self.last_flush = Instant::now();
          }
          Err(e) => {
            total_errors += self.batch_buffer.len();
            error!(
              component = %component_name,
              error = %e,
              batch_size = self.batch_buffer.len(),
              "Failed to insert batch, dropping batch"
            );

            // Handle error according to strategy
            let stream_error = StreamError::new(
              e.to_string().into(),
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

            match handle_error_strategy(&error_strategy, &stream_error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  "Stopping due to batch insert error"
                );
                break;
              }
              ErrorAction::Skip => {
                warn!(
                  component = %component_name,
                  "Skipping batch due to error"
                );
                self.batch_buffer.clear();
                self.last_flush = Instant::now();
                continue;
              }
              ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  "Retry not yet implemented for batch inserts, skipping batch"
                );
                self.batch_buffer.clear();
                self.last_flush = Instant::now();
                continue;
              }
            }
          }
        }
      }
    }

    // Flush any remaining rows in buffer
    if !self.batch_buffer.is_empty() {
      match flush_batch(&pool, &db_config, &mut self.batch_buffer, &component_name).await {
        Ok(count) => {
          total_inserted += count;
        }
        Err(e) => {
          total_errors += self.batch_buffer.len();
          error!(
            component = %component_name,
            error = %e,
            batch_size = self.batch_buffer.len(),
            "Failed to insert final batch"
          );
        }
      }
    }

    // Store pool back for potential reuse
    self.pool = Some(pool);

    if total_inserted > 0 {
      tracing::info!(
        component = %component_name,
        inserted = total_inserted,
        errors = total_errors,
        "Finished consuming database stream"
      );
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<DatabaseRow>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<DatabaseRow> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<DatabaseRow> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<DatabaseRow>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<DatabaseRow>) -> ErrorContext<DatabaseRow> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
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

  // Get column names from first row (or use mapping)
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

  // Build INSERT query
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

  // Execute in transaction if configured
  if config.use_transactions {
    let mut tx = pool.begin().await?;
    let mut inserted = 0;

    for row in rows {
      let mut query_builder = sqlx::query(&query);

      // Bind parameters in column order
      for col in &columns {
        let field_name = if let Some(ref mapping) = config.column_mapping {
          // Reverse lookup: find the field name that maps to this column
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

      // Commit transaction if transaction_size is set
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
