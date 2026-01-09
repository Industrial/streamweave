//! Comprehensive tests for DbSqliteConsumer

use futures::{Stream, StreamExt, stream};
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;
use streamweave::consumers::DbSqliteConsumer;
use streamweave::db::{DatabaseConsumerConfig, DatabaseRow, DatabaseType};
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, Input};

async fn setup_test_db() -> String {
  let db_path = format!(
    "sqlite:{}",
    tempfile::NamedTempFile::new()
      .unwrap()
      .path()
      .to_str()
      .unwrap()
  );
  let pool = SqlitePoolOptions::new()
    .max_connections(1)
    .connect(&db_path)
    .await
    .unwrap();

  sqlx::query("CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT, age INTEGER, active INTEGER, score REAL)")
    .execute(&pool)
    .await
    .unwrap();

  pool.close().await;
  db_path
}

async fn verify_rows(db_path: &str, expected_count: usize) {
  let pool = SqlitePoolOptions::new()
    .max_connections(1)
    .connect(db_path)
    .await
    .unwrap();

  let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_table")
    .fetch_one(&pool)
    .await
    .unwrap();

  assert_eq!(count as usize, expected_count);
  pool.close().await;
}

#[tokio::test]
async fn test_sqlite_consumer_new() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let consumer = DbSqliteConsumer::new(config.clone());
  assert_eq!(consumer.db_config().connection_url, config.connection_url);
}

#[tokio::test]
async fn test_sqlite_consumer_with_error_strategy() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let consumer = DbSqliteConsumer::new(config).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config().error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_sqlite_consumer_with_name() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let consumer = DbSqliteConsumer::new(config).with_name("test_consumer".to_string());
  assert_eq!(consumer.config().name, "test_consumer");
}

#[tokio::test]
async fn test_sqlite_consumer_clone() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let consumer = DbSqliteConsumer::new(config).with_name("test".to_string());
  let cloned = consumer.clone();
  assert_eq!(consumer.config().name, cloned.config().name);
}

#[tokio::test]
async fn test_sqlite_consumer_consume_single_row() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(1);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Number(30.into()));
  fields.insert("active".to_string(), Value::Number(1.into()));
  fields.insert(
    "score".to_string(),
    Value::Number(serde_json::Number::from_f64(95.5).unwrap()),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));

  consumer.consume(stream).await;
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_multiple_rows() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut rows = Vec::new();
  for i in 1..=5 {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Number(i.into()));
    fields.insert("name".to_string(), Value::String(format!("User{}", i)));
    fields.insert("age".to_string(), Value::Number((20 + i).into()));
    fields.insert("active".to_string(), Value::Number(1.into()));
    fields.insert(
      "score".to_string(),
      Value::Number(serde_json::Number::from_f64(80.0 + i as f64).unwrap()),
    );
    rows.push(DatabaseRow::new(fields));
  }

  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(rows));
  consumer.consume(stream).await;
  verify_rows(&db_path, 5).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_batch_size() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(3);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut rows = Vec::new();
  for i in 1..=7 {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Number(i.into()));
    fields.insert("name".to_string(), Value::String(format!("User{}", i)));
    fields.insert("age".to_string(), Value::Number((20 + i).into()));
    fields.insert("active".to_string(), Value::Number(1.into()));
    fields.insert(
      "score".to_string(),
      Value::Number(serde_json::Number::from_f64(80.0 + i as f64).unwrap()),
    );
    rows.push(DatabaseRow::new(fields));
  }

  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(rows));
  consumer.consume(stream).await;
  verify_rows(&db_path, 7).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_batch_timeout() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(100)
    .with_batch_timeout(Duration::from_millis(100));

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Number(30.into()));
  fields.insert("active".to_string(), Value::Number(1.into()));
  fields.insert(
    "score".to_string(),
    Value::Number(serde_json::Number::from_f64(95.5).unwrap()),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));

  consumer.consume(stream).await;
  tokio::time::sleep(Duration::from_millis(150)).await; // Wait for timeout
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_transactions() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10)
    .with_transactions(true);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut rows = Vec::new();
  for i in 1..=5 {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Number(i.into()));
    fields.insert("name".to_string(), Value::String(format!("User{}", i)));
    fields.insert("age".to_string(), Value::Number((20 + i).into()));
    fields.insert("active".to_string(), Value::Number(1.into()));
    fields.insert(
      "score".to_string(),
      Value::Number(serde_json::Number::from_f64(80.0 + i as f64).unwrap()),
    );
    rows.push(DatabaseRow::new(fields));
  }

  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(rows));
  consumer.consume(stream).await;
  verify_rows(&db_path, 5).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_without_transactions() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10)
    .with_transactions(false);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut rows = Vec::new();
  for i in 1..=5 {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Number(i.into()));
    fields.insert("name".to_string(), Value::String(format!("User{}", i)));
    fields.insert("age".to_string(), Value::Number((20 + i).into()));
    fields.insert("active".to_string(), Value::Number(1.into()));
    fields.insert(
      "score".to_string(),
      Value::Number(serde_json::Number::from_f64(80.0 + i as f64).unwrap()),
    );
    rows.push(DatabaseRow::new(fields));
  }

  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(rows));
  consumer.consume(stream).await;
  verify_rows(&db_path, 5).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_transaction_size() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10)
    .with_transactions(true)
    .with_transaction_size(Some(3));

  let mut consumer = DbSqliteConsumer::new(config);

  let mut rows = Vec::new();
  for i in 1..=7 {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Number(i.into()));
    fields.insert("name".to_string(), Value::String(format!("User{}", i)));
    fields.insert("age".to_string(), Value::Number((20 + i).into()));
    fields.insert("active".to_string(), Value::Number(1.into()));
    fields.insert(
      "score".to_string(),
      Value::Number(serde_json::Number::from_f64(80.0 + i as f64).unwrap()),
    );
    rows.push(DatabaseRow::new(fields));
  }

  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(rows));
  consumer.consume(stream).await;
  verify_rows(&db_path, 7).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_column_mapping() {
  let db_path = setup_test_db().await;
  let mut mapping = HashMap::new();
  mapping.insert("user_id".to_string(), "id".to_string());
  mapping.insert("user_name".to_string(), "name".to_string());
  mapping.insert("user_age".to_string(), "age".to_string());
  mapping.insert("is_active".to_string(), "active".to_string());
  mapping.insert("user_score".to_string(), "score".to_string());

  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_column_mapping(mapping)
    .with_batch_size(10);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("user_id".to_string(), Value::Number(1.into()));
  fields.insert("user_name".to_string(), Value::String("Alice".to_string()));
  fields.insert("user_age".to_string(), Value::Number(30.into()));
  fields.insert("is_active".to_string(), Value::Number(1.into()));
  fields.insert(
    "user_score".to_string(),
    Value::Number(serde_json::Number::from_f64(95.5).unwrap()),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));
  consumer.consume(stream).await;
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_custom_query() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_insert_query(
      "INSERT INTO test_table (id, name, age, active, score) VALUES (?, ?, ?, ?, ?)",
    )
    .with_batch_size(10);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Number(30.into()));
  fields.insert("active".to_string(), Value::Number(1.into()));
  fields.insert(
    "score".to_string(),
    Value::Number(serde_json::Number::from_f64(95.5).unwrap()),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));
  consumer.consume(stream).await;
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_null_values() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Null);
  fields.insert("active".to_string(), Value::Number(1.into()));
  fields.insert("score".to_string(), Value::Null);

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));
  consumer.consume(stream).await;
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_bool_values() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Number(30.into()));
  fields.insert("active".to_string(), Value::Bool(true));
  fields.insert(
    "score".to_string(),
    Value::Number(serde_json::Number::from_f64(95.5).unwrap()),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));
  consumer.consume(stream).await;
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_with_json_values() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(10);

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Number(30.into()));
  fields.insert("active".to_string(), Value::Number(1.into()));
  fields.insert(
    "score".to_string(),
    Value::Object({
      let mut map = serde_json::Map::new();
      map.insert("math".to_string(), Value::Number(95.into()));
      map.insert("science".to_string(), Value::Number(96.into()));
      map
    }),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));
  consumer.consume(stream).await;
  verify_rows(&db_path, 1).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_empty_stream() {
  let db_path = setup_test_db().await;
  let config = DatabaseConsumerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let mut consumer = DbSqliteConsumer::new(config);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::empty());
  consumer.consume(stream).await;
  verify_rows(&db_path, 0).await;
}

#[tokio::test]
async fn test_sqlite_consumer_consume_invalid_connection() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite:/nonexistent/path/db.sqlite")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let mut consumer = DbSqliteConsumer::new(config);

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  fields.insert("name".to_string(), Value::String("Alice".to_string()));
  fields.insert("age".to_string(), Value::Number(30.into()));
  fields.insert("active".to_string(), Value::Number(1.into()));
  fields.insert(
    "score".to_string(),
    Value::Number(serde_json::Number::from_f64(95.5).unwrap()),
  );

  let row = DatabaseRow::new(fields);
  let stream: Pin<Box<dyn Stream<Item = DatabaseRow> + Send>> = Box::pin(stream::iter(vec![row]));
  // Should not panic, but drop items
  consumer.consume(stream).await;
}

#[tokio::test]
async fn test_sqlite_consumer_config_methods() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_batch_size(1000)
    .with_max_connections(5);

  let consumer = DbSqliteConsumer::new(config.clone());
  let db_config = consumer.db_config();

  assert_eq!(db_config.connection_url, config.connection_url);
  assert_eq!(db_config.database_type, config.database_type);
  assert_eq!(db_config.table_name, config.table_name);
  assert_eq!(db_config.batch_size, config.batch_size);
  assert_eq!(db_config.max_connections, config.max_connections);
}

#[tokio::test]
async fn test_sqlite_consumer_set_config() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let mut consumer = DbSqliteConsumer::new(config);
  let new_config = streamweave::ConsumerConfig::default();
  consumer.set_config(new_config.clone());
  assert_eq!(consumer.config(), &new_config);
}

#[tokio::test]
async fn test_sqlite_consumer_get_config_mut() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let mut consumer = DbSqliteConsumer::new(config);
  let config_mut = consumer.get_config_mut();
  config_mut.name = "modified".to_string();
  assert_eq!(consumer.config().name, "modified");
}

#[tokio::test]
async fn test_sqlite_consumer_handle_error() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let consumer = DbSqliteConsumer::new(config);
  let error = streamweave::error::StreamError::new(
    "test error".to_string().into(),
    streamweave::error::ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "DbSqliteConsumer".to_string(),
    },
    streamweave::error::ComponentInfo {
      name: "test".to_string(),
      type_name: "DbSqliteConsumer".to_string(),
    },
  );

  let action = consumer.handle_error(&error);
  assert!(matches!(action, streamweave::error::ErrorAction::Stop));
}

#[tokio::test]
async fn test_sqlite_consumer_create_error_context() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_name("test_consumer".to_string());

  let consumer = DbSqliteConsumer::new(config);
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), Value::Number(1.into()));
  let row = DatabaseRow::new(fields);
  let context = consumer.create_error_context(Some(row.clone()));
  assert_eq!(context.component_name, "test_consumer");
  assert_eq!(context.item, Some(row));
}

#[tokio::test]
async fn test_sqlite_consumer_component_info() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table")
    .with_name("test_consumer".to_string());

  let consumer = DbSqliteConsumer::new(config);
  let info = consumer.component_info();
  assert_eq!(info.name, "test_consumer");
  assert_eq!(
    info.type_name,
    "streamweave::db_sqlite::sqlite_consumer::DbSqliteConsumer"
  );
}
