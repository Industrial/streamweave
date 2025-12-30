//! Comprehensive tests for SqliteProducer

use futures::StreamExt;
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::time::Duration;
use streamweave_database::{DatabaseProducerConfig, DatabaseRow, DatabaseType};
use streamweave_database_sqlite::SqliteProducer;
use streamweave_error::ErrorStrategy;
use tempfile;

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

  sqlx::query("CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT, age INTEGER, active BOOLEAN, score REAL, data BLOB)")
    .execute(&pool)
    .await
    .unwrap();

  sqlx::query("INSERT INTO test_table (id, name, age, active, score, data) VALUES (1, 'Alice', 30, 1, 95.5, ?)")
    .bind(b"binary_data".to_vec())
    .execute(&pool)
    .await
    .unwrap();

  sqlx::query(
    "INSERT INTO test_table (id, name, age, active, score, data) VALUES (2, 'Bob', 25, 0, 87.5, ?)",
  )
  .bind(b"more_data".to_vec())
  .execute(&pool)
  .await
  .unwrap();

  sqlx::query("INSERT INTO test_table (id, name, age, active, score, data) VALUES (3, 'Charlie', 35, 1, 92.0, NULL)")
    .execute(&pool)
    .await
    .unwrap();

  pool.close().await;
  db_path
}

#[tokio::test]
async fn test_sqlite_producer_new() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let producer = SqliteProducer::new(config.clone());
  assert_eq!(producer.db_config().connection_url, config.connection_url);
}

#[tokio::test]
async fn test_sqlite_producer_with_error_strategy() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let producer = SqliteProducer::new(config).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config().error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_sqlite_producer_with_name() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let producer = SqliteProducer::new(config).with_name("test_producer".to_string());
  assert_eq!(producer.config().name, Some("test_producer".to_string()));
}

#[tokio::test]
async fn test_sqlite_producer_clone() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let producer = SqliteProducer::new(config).with_name("test".to_string());
  let cloned = producer.clone();
  assert_eq!(producer.config().name, cloned.config().name);
}

#[tokio::test]
async fn test_sqlite_producer_produce_simple_query() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, age FROM test_table ORDER BY id");

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 3);
  assert_eq!(rows[0].get("id"), Some(&Value::Number(1.into())));
  assert_eq!(
    rows[0].get("name"),
    Some(&Value::String("Alice".to_string()))
  );
  assert_eq!(rows[0].get("age"), Some(&Value::Number(30.into())));
}

#[tokio::test]
async fn test_sqlite_producer_produce_all_types() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM test_table WHERE id = 1");

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();

  assert_eq!(row.get("id"), Some(&Value::Number(1.into())));
  assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
  assert_eq!(row.get("age"), Some(&Value::Number(30.into())));
  assert_eq!(row.get("active"), Some(&Value::Number(1.into()))); // SQLite boolean as integer
  assert_eq!(
    row.get("score"),
    Some(&Value::Number(serde_json::Number::from_f64(95.5).unwrap()))
  );
  // BLOB is base64 encoded
  assert!(row.get("data").is_some());
  let data = row.get("data").unwrap().as_str().unwrap();
  assert!(data.len() > 0); // Base64 encoded binary data
}

#[tokio::test]
async fn test_sqlite_producer_produce_with_null() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, data FROM test_table WHERE id = 3");

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();

  assert_eq!(row.get("id"), Some(&Value::Number(3.into())));
  assert_eq!(row.get("name"), Some(&Value::String("Charlie".to_string())));
  // NULL values may or may not be present depending on implementation
}

#[tokio::test]
async fn test_sqlite_producer_produce_with_parameters() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM test_table WHERE id = ? AND age > ?")
    .with_parameter(Value::Number(1.into()))
    .with_parameter(Value::Number(20.into()));

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 1);
  assert_eq!(rows[0].get("id"), Some(&Value::Number(1.into())));
}

#[tokio::test]
async fn test_sqlite_producer_produce_with_string_parameter() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM test_table WHERE name = ?")
    .with_parameter(Value::String("Alice".to_string()));

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();

  assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
}

#[tokio::test]
async fn test_sqlite_producer_produce_with_bool_parameter() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM test_table WHERE active = ?")
    .with_parameter(Value::Bool(true));

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 2); // Alice and Charlie are active
}

#[tokio::test]
async fn test_sqlite_producer_produce_with_null_parameter() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM test_table WHERE data IS ?")
    .with_parameter(Value::Null);

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 1); // Only Charlie has NULL data
  assert_eq!(rows[0].get("id"), Some(&Value::Number(3.into())));
}

#[tokio::test]
async fn test_sqlite_producer_produce_with_json_parameter() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT ? as json_data")
    .with_parameter(Value::Object({
      let mut map = serde_json::Map::new();
      map.insert("key".to_string(), Value::String("value".to_string()));
      map
    }));

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();

  // JSON is serialized as string in SQLite
  assert!(row.get("json_data").is_some());
}

#[tokio::test]
async fn test_sqlite_producer_produce_empty_result() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM test_table WHERE id = 999");

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await;
  assert!(row.is_none());
}

#[tokio::test]
async fn test_sqlite_producer_produce_invalid_connection() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite:/nonexistent/path/db.sqlite")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  // Should produce empty stream on connection error
  let row = stream.next().await;
  assert!(row.is_none());
}

#[tokio::test]
async fn test_sqlite_producer_produce_invalid_query() {
  let db_path = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&db_path)
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM nonexistent_table");

  let mut producer = SqliteProducer::new(config);
  let mut stream = producer.produce();
  // Should produce empty stream on query error
  let row = stream.next().await;
  assert!(row.is_none());
}

#[tokio::test]
async fn test_sqlite_producer_config_methods() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1")
    .with_max_connections(5)
    .with_fetch_size(500)
    .with_ssl(false);

  let producer = SqliteProducer::new(config.clone());
  let db_config = producer.db_config();

  assert_eq!(db_config.connection_url, config.connection_url);
  assert_eq!(db_config.database_type, config.database_type);
  assert_eq!(db_config.query, config.query);
  assert_eq!(db_config.max_connections, config.max_connections);
  assert_eq!(db_config.fetch_size, config.fetch_size);
}

#[tokio::test]
async fn test_sqlite_producer_set_config() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let mut producer = SqliteProducer::new(config);
  let new_config = streamweave::ProducerConfig::default();
  producer.set_config(new_config.clone());
  assert_eq!(producer.config(), &new_config);
}

#[tokio::test]
async fn test_sqlite_producer_get_config_mut() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT 1");

  let mut producer = SqliteProducer::new(config);
  let config_mut = producer.get_config_mut();
  config_mut.name = Some("modified".to_string());
  assert_eq!(producer.config().name, Some("modified".to_string()));
}
