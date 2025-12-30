use async_stream::stream;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::{Stream, StreamExt};
use sqlx::Column;
use std::collections::HashMap;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_database::{DatabaseProducerConfig, DatabaseRow};
use streamweave_error::ErrorStrategy;
// Error types not used in producer implementation
use tracing::{error, warn};

/// A producer that executes PostgreSQL queries and streams the results.
///
/// This producer uses connection pooling and provides cursor-based iteration
/// for large result sets to keep memory usage bounded.
pub struct PostgresProducer {
  /// Producer configuration.
  pub config: ProducerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseProducerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::PgPool>,
}

impl PostgresProducer {
  /// Creates a new PostgreSQL producer with the given configuration.
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

impl Clone for PostgresProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
    }
  }
}

impl Output for PostgresProducer {
  type Output = DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = DatabaseRow> + Send>>;
}

#[async_trait]
impl Producer for PostgresProducer {
  type OutputPorts = (DatabaseRow,);

  /// Produces a stream of database rows from the configured query.
  fn produce(&mut self) -> Self::OutputStream {
    let db_config = self.db_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "postgres_producer".to_string());
    let _error_strategy = self.config.error_strategy.clone();

    // Take the pool, creating it synchronously if needed
    let pool_option = self.pool.take();

    Box::pin(stream! {
      // Create connection pool if not already created
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

      // Execute query and stream results
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

      // Stream rows
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
) -> Result<sqlx::PgPool, Box<dyn std::error::Error + Send + Sync>> {
  let pool_options = sqlx::postgres::PgPoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .acquire_timeout(config.connect_timeout)
    .idle_timeout(config.idle_timeout)
    .max_lifetime(config.max_lifetime);

  let pool = pool_options.connect(&config.connection_url).await?;
  Ok(pool)
}

async fn execute_query<'a>(
  pool: &'a sqlx::PgPool,
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
  // Use sqlx::query for runtime queries (supports dynamic parameters)
  let mut query = sqlx::query(&config.query);

  // Bind parameters dynamically
  for param in &config.parameters {
    query = bind_parameter_postgres(query, param)?;
  }

  // Execute query and map rows - cursor-based streaming for large results
  let row_stream = query.fetch(pool).map(|row_result| {
    row_result
      .map(|row: sqlx::postgres::PgRow| convert_row_to_database_row(&row))
      .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
  });

  Ok(Box::pin(row_stream))
}

fn bind_parameter_postgres<'q>(
  query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
  param: &serde_json::Value,
) -> Result<
  sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
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
        return Err("Unsupported number type for PostgreSQL parameter".into());
      }
    }
    serde_json::Value::String(s) => query.bind(s.clone()),
    serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
      // Serialize complex types to JSON string for PostgreSQL JSON/JSONB columns
      let json_str = serde_json::to_string(param)?;
      query.bind(json_str)
    }
  };
  Ok(bound_query)
}

fn convert_row_to_database_row(row: &sqlx::postgres::PgRow) -> DatabaseRow {
  use sqlx::Row;
  let mut fields = HashMap::new();

  for (idx, column) in row.columns().iter().enumerate() {
    let value = match format!("{}", column.type_info()).as_str() {
      "BOOL" | "BOOLEAN" => row
        .try_get::<bool, _>(idx)
        .ok()
        .map(serde_json::Value::Bool),
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
        .map(serde_json::Value::String),
      "BYTEA" => row
        .try_get::<Vec<u8>, _>(idx)
        .ok()
        .map(|v| serde_json::Value::String(BASE64.encode(v))),
      _ => row
        .try_get::<String, _>(idx)
        .ok()
        .map(serde_json::Value::String)
        .or_else(|| row.try_get::<serde_json::Value, _>(idx).ok()),
    };

    if let Some(val) = value {
      fields.insert(column.name().to_string(), val);
    }
  }

  DatabaseRow::new(fields)
}
