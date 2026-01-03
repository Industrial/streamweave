use async_stream::stream;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::{Stream, StreamExt};
use sqlx::Column;
use std::collections::HashMap;
use std::pin::Pin;
use streamweave::error::ErrorStrategy;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_database::{DatabaseProducerConfig, DatabaseRow};
// Error types not used in producer implementation
use tracing::{error, warn};

/// A producer that executes SQLite queries and streams the results.
///
/// This producer uses connection pooling and provides cursor-based iteration
/// for large result sets to keep memory usage bounded.
pub struct SqliteProducer {
  /// Producer configuration.
  pub config: ProducerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseProducerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::SqlitePool>,
}

impl SqliteProducer {
  /// Creates a new SQLite producer with the given configuration.
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

impl Clone for SqliteProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
    }
  }
}

impl Output for SqliteProducer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Producer for SqliteProducer {
  type OutputPorts = (DatabaseRow,);

  /// Produces a stream of database rows from the configured query.
  fn produce(&mut self) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "sqlite_producer".to_string());
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

async fn execute_query<'a>(
  pool: &'a sqlx::SqlitePool,
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
    query = bind_parameter_sqlite(query, param)?;
  }

  let row_stream = query.fetch(pool).map(|row_result| {
    row_result
      .map(|row: sqlx::sqlite::SqliteRow| convert_sqlite_row_to_database_row(&row))
      .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
  });

  Ok(Box::pin(row_stream))
}

fn bind_parameter_sqlite<'q>(
  query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  param: &serde_json::Value,
) -> Result<
  sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  Box<dyn std::error::Error + Send + Sync>,
> {
  let bound_query = match param {
    serde_json::Value::Null => query.bind(None::<Option<String>>),
    serde_json::Value::Bool(b) => query.bind(if *b { 1i64 } else { 0i64 }), // SQLite uses integer for boolean
    serde_json::Value::Number(n) => {
      if let Some(i) = n.as_i64() {
        query.bind(i)
      } else if let Some(f) = n.as_f64() {
        query.bind(f)
      } else {
        return Err("Unsupported number type for SQLite parameter".into());
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

fn convert_sqlite_row_to_database_row(row: &sqlx::sqlite::SqliteRow) -> DatabaseRow {
  use sqlx::Row;
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    // SQLite TypeInfo doesn't have a name() method, so we try common types
    let value = row
      .try_get::<i64, _>(idx)
      .ok()
      .map(|v| serde_json::Value::Number(v.into()))
      .or_else(|| {
        row
          .try_get::<f64, _>(idx)
          .ok()
          .map(|v| serde_json::Value::Number(serde_json::Number::from_f64(v).unwrap()))
      })
      .or_else(|| {
        row
          .try_get::<String, _>(idx)
          .ok()
          .map(serde_json::Value::String)
      })
      .or_else(|| {
        row
          .try_get::<Vec<u8>, _>(idx)
          .ok()
          .map(|v| serde_json::Value::String(BASE64.encode(v)))
      });

    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }

  DatabaseRow::new(fields)
}
