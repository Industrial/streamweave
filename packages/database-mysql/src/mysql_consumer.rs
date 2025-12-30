use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::time::Instant;
use streamweave::{Consumer, ConsumerConfig, Input};
use streamweave_database::{DatabaseConsumerConfig, DatabaseRow};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::{error, warn};

/// A consumer that writes database rows to a MySQL table.
///
/// This consumer supports batch insertion for performance and transaction management.
pub struct MysqlConsumer {
  /// Consumer configuration.
  pub config: ConsumerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseConsumerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::MySqlPool>,
  /// Buffer for batch insertion.
  pub(crate) batch_buffer: Vec<DatabaseRow>,
  /// Timestamp of last batch flush.
  pub(crate) last_flush: Instant,
}

impl MysqlConsumer {
  /// Creates a new MySQL consumer with the given configuration.
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

impl Clone for MysqlConsumer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None,
      batch_buffer: Vec::new(),
      last_flush: Instant::now(),
    }
  }
}

impl Input for MysqlConsumer {
  type Input = DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Consumer for MysqlConsumer {
  type InputPorts = (DatabaseRow,);

  /// Consumes a stream of database rows and inserts them into the configured table.
  async fn consume(&mut self, mut stream: Self::InputStream) {
    let component_name = if self.config.name.is_empty() {
      "mysql_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let db_config = self.db_config.clone();

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
      self.batch_buffer.push(row);

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
) -> Result<sqlx::MySqlPool, Box<dyn std::error::Error + Send + Sync + 'static>> {
  let pool_options = sqlx::mysql::MySqlPoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(config.idle_timeout)
    .max_lifetime(config.max_lifetime);

  let pool = pool_options.connect(&config.connection_url).await?;
  Ok(pool)
}

async fn flush_batch(
  pool: &sqlx::MySqlPool,
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
    let placeholders = vec!["?"; columns.len()].join(", ");
    format!(
      "INSERT INTO {} ({}) VALUES ({})",
      config.table_name,
      columns.join(", "),
      placeholders
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
        query_builder = bind_value_mysql(query_builder, value)?;
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
        query_builder = bind_value_mysql(query_builder, value)?;
      }

      query_builder.execute(pool).await?;
      inserted += 1;
    }

    Ok(inserted)
  }
}

fn bind_value_mysql<'q>(
  query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
  value: Option<&serde_json::Value>,
) -> Result<
  sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
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
        return Err("Unsupported number type for MySQL parameter".into());
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
