use super::super::producers::database_producer::{
  DatabaseConsumerConfig, DatabasePool, DatabaseRow, DatabaseType,
};
use super::database_consumer::DatabaseConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use std::time::Instant;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::{error, warn};

#[async_trait]
impl Consumer for DatabaseConsumer {
  type InputPorts = (DatabaseRow,);

  /// Consumes a stream of database rows and inserts them into the configured table.
  ///
  /// # Error Handling
  ///
  /// - Connection errors are handled according to the error strategy.
  /// - Insert errors trigger retries based on the error strategy.
  /// - Row processing errors are handled per the error strategy.
  async fn consume(&mut self, mut stream: Self::InputStream) {
    let component_name = if self.config.name.is_empty() {
      "database_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let db_config = self.db_config.clone();

    // Create connection pool if not already created
    let pool = match self.pool.take() {
      Some(p) => p,
      None => match create_consumer_pool(&db_config).await {
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

async fn create_consumer_pool(
  config: &DatabaseConsumerConfig,
) -> Result<DatabasePool, Box<dyn std::error::Error + Send + Sync + 'static>> {
  match config.database_type {
    DatabaseType::Postgres => {
      let pool_options = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime);

      let pool = pool_options.connect(&config.connection_url).await?;
      Ok(DatabasePool::Postgres(pool))
    }
    DatabaseType::Mysql => {
      let pool_options = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime);

      let pool = pool_options.connect(&config.connection_url).await?;
      Ok(DatabasePool::Mysql(pool))
    }
    DatabaseType::Sqlite => {
      let pool_options = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.connect_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime);

      let pool = pool_options.connect(&config.connection_url).await?;
      Ok(DatabasePool::Sqlite(pool))
    }
  }
}

async fn flush_batch(
  pool: &DatabasePool,
  config: &DatabaseConsumerConfig,
  batch: &mut Vec<DatabaseRow>,
  component_name: &str,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
  if batch.is_empty() {
    return Ok(0);
  }

  let rows = std::mem::take(batch);

  match pool {
    DatabasePool::Postgres(pg_pool) => {
      insert_batch_postgres(pg_pool, config, rows, component_name).await
    }
    DatabasePool::Mysql(mysql_pool) => {
      insert_batch_mysql(mysql_pool, config, rows, component_name).await
    }
    DatabasePool::Sqlite(sqlite_pool) => {
      insert_batch_sqlite(sqlite_pool, config, rows, component_name).await
    }
  }
}

async fn insert_batch_postgres(
  pool: &sqlx::PgPool,
  config: &DatabaseConsumerConfig,
  rows: Vec<DatabaseRow>,
  _component_name: &str,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
  if rows.is_empty() {
    return Ok(0);
  }

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

async fn insert_batch_mysql(
  pool: &sqlx::MySqlPool,
  config: &DatabaseConsumerConfig,
  rows: Vec<DatabaseRow>,
  _component_name: &str,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
  if rows.is_empty() {
    return Ok(0);
  }

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

async fn insert_batch_sqlite(
  pool: &sqlx::SqlitePool,
  config: &DatabaseConsumerConfig,
  rows: Vec<DatabaseRow>,
  _component_name: &str,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync + 'static>> {
  if rows.is_empty() {
    return Ok(0);
  }

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
        query_builder = bind_value_sqlite(query_builder, value)?;
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
        query_builder = bind_value_sqlite(query_builder, value)?;
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

fn bind_value_sqlite<'q>(
  query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  value: Option<&serde_json::Value>,
) -> Result<
  sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
  Box<dyn std::error::Error + Send + Sync + 'static>,
> {
  let bound_query = match value {
    None | Some(serde_json::Value::Null) => query.bind(None::<Option<String>>),
    Some(serde_json::Value::Bool(b)) => query.bind(if *b { 1i64 } else { 0i64 }),
    Some(serde_json::Value::Number(n)) => {
      if let Some(i) = n.as_i64() {
        query.bind(i)
      } else if let Some(f) = n.as_f64() {
        query.bind(f)
      } else {
        return Err("Unsupported number type for SQLite parameter".into());
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

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use sqlx::SqlitePool;
  use std::collections::HashMap;
  use std::time::Duration;

  async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:?cache=shared")
      .await
      .unwrap();

    sqlx::query(
      r#"
      CREATE TABLE users (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT,
        age INTEGER,
        active INTEGER,
        balance REAL
      )
      "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
  }

  #[tokio::test]
  async fn test_database_consumer_config_default() {
    let config = DatabaseConsumerConfig::default();
    assert_eq!(config.database_type, DatabaseType::Postgres);
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.batch_timeout, Duration::from_secs(5));
    assert!(config.use_transactions);
    assert_eq!(config.max_connections, 10);
  }

  #[tokio::test]
  async fn test_database_consumer_config_builder() {
    let config = DatabaseConsumerConfig::default()
      .with_connection_url("postgresql://user:pass@localhost/db")
      .with_database_type(DatabaseType::Mysql)
      .with_table_name("users")
      .with_batch_size(50)
      .with_batch_timeout(Duration::from_secs(10))
      .with_transactions(false)
      .with_max_connections(20);

    assert_eq!(config.connection_url, "postgresql://user:pass@localhost/db");
    assert_eq!(config.database_type, DatabaseType::Mysql);
    assert_eq!(config.table_name, "users");
    assert_eq!(config.batch_size, 50);
    assert_eq!(config.batch_timeout, Duration::from_secs(10));
    assert!(!config.use_transactions);
    assert_eq!(config.max_connections, 20);
  }

  #[tokio::test]
  async fn test_database_consumer_basic_insert() {
    let pool = setup_test_db().await;

    let config = DatabaseConsumerConfig::default()
      .with_connection_url("sqlite::memory:?cache=shared")
      .with_database_type(DatabaseType::Sqlite)
      .with_table_name("users")
      .with_batch_size(2);

    let mut consumer = DatabaseConsumer::new(config);
    consumer.pool = Some(DatabasePool::Sqlite(pool.clone()));

    // Create test rows
    let mut row1_fields = HashMap::new();
    row1_fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
    row1_fields.insert(
      "name".to_string(),
      serde_json::Value::String("Alice".to_string()),
    );
    row1_fields.insert(
      "email".to_string(),
      serde_json::Value::String("alice@example.com".to_string()),
    );
    row1_fields.insert("age".to_string(), serde_json::Value::Number(30.into()));
    row1_fields.insert("active".to_string(), serde_json::Value::Number(1.into()));
    row1_fields.insert(
      "balance".to_string(),
      serde_json::Value::Number(serde_json::Number::from_f64(1000.50).unwrap()),
    );

    let mut row2_fields = HashMap::new();
    row2_fields.insert("id".to_string(), serde_json::Value::Number(2.into()));
    row2_fields.insert(
      "name".to_string(),
      serde_json::Value::String("Bob".to_string()),
    );
    row2_fields.insert(
      "email".to_string(),
      serde_json::Value::String("bob@example.com".to_string()),
    );
    row2_fields.insert("age".to_string(), serde_json::Value::Number(25.into()));
    row2_fields.insert("active".to_string(), serde_json::Value::Number(1.into()));
    row2_fields.insert(
      "balance".to_string(),
      serde_json::Value::Number(serde_json::Number::from_f64(750.25).unwrap()),
    );

    let rows = vec![DatabaseRow::new(row1_fields), DatabaseRow::new(row2_fields)];

    let input_stream = Box::pin(stream::iter(rows));
    consumer.consume(input_stream).await;

    // Verify data was inserted
    let result = sqlx::query("SELECT COUNT(*) as count FROM users")
      .fetch_one(&pool)
      .await
      .unwrap();
    let count: i64 = sqlx::Row::get(&result, 0);
    assert_eq!(count, 2);
  }

  #[tokio::test]
  async fn test_database_consumer_batch_insert() {
    let pool = setup_test_db().await;

    let config = DatabaseConsumerConfig::default()
      .with_connection_url("sqlite::memory:?cache=shared")
      .with_database_type(DatabaseType::Sqlite)
      .with_table_name("users")
      .with_batch_size(3);

    let mut consumer = DatabaseConsumer::new(config);
    consumer.pool = Some(DatabasePool::Sqlite(pool.clone()));

    // Create 5 rows (should batch into 3 + 2)
    let mut rows = Vec::new();
    for i in 1..=5 {
      let mut fields = HashMap::new();
      fields.insert("id".to_string(), serde_json::Value::Number(i.into()));
      fields.insert(
        "name".to_string(),
        serde_json::Value::String(format!("User{}", i)),
      );
      fields.insert(
        "email".to_string(),
        serde_json::Value::String(format!("user{}@example.com", i)),
      );
      fields.insert(
        "age".to_string(),
        serde_json::Value::Number((20 + i).into()),
      );
      fields.insert("active".to_string(), serde_json::Value::Number(1.into()));
      fields.insert(
        "balance".to_string(),
        serde_json::Value::Number(serde_json::Number::from_f64(1000.0 + i as f64).unwrap()),
      );
      rows.push(DatabaseRow::new(fields));
    }

    let input_stream = Box::pin(stream::iter(rows));
    consumer.consume(input_stream).await;

    // Verify all 5 rows were inserted
    let result = sqlx::query("SELECT COUNT(*) as count FROM users")
      .fetch_one(&pool)
      .await
      .unwrap();
    let count: i64 = sqlx::Row::get(&result, 0);
    assert_eq!(count, 5);
  }
}
