//! Comprehensive tests for MysqlProducer
//!
//! These tests require a MySQL database. Set MYSQL_URL environment variable
//! or use default: mysql://root:password@localhost:3306/test

use futures::StreamExt;
use serde_json::Value;
use sqlx::mysql::MySqlPoolOptions;
use std::collections::HashMap;
use std::time::Duration;
use streamweave_database::{DatabaseProducerConfig, DatabaseRow, DatabaseType};
use streamweave_database_mysql::MysqlProducer;
use streamweave_error::ErrorStrategy;

fn get_mysql_url() -> String {
  std::env::var("MYSQL_URL")
    .unwrap_or_else(|_| "mysql://root:password@localhost:3306/test".to_string())
}

async fn setup_test_db() -> String {
  let url = get_mysql_url();
  let pool = MySqlPoolOptions::new()
    .max_connections(1)
    .connect(&url)
    .await
    .expect("Failed to connect to MySQL");

  sqlx::query("DROP TABLE IF EXISTS test_table")
    .execute(&pool)
    .await
    .ok();

  sqlx::query(
    "CREATE TABLE test_table (
      id INT PRIMARY KEY,
      name VARCHAR(100),
      age INT,
      active BOOLEAN,
      score DOUBLE,
      data BLOB
    )",
  )
  .execute(&pool)
  .await
  .expect("Failed to create table");

  sqlx::query("INSERT INTO test_table (id, name, age, active, score, data) VALUES (1, 'Alice', 30, TRUE, 95.5, ?)")
    .bind(b"binary_data".to_vec())
    .execute(&pool)
    .await
    .expect("Failed to insert");

  sqlx::query("INSERT INTO test_table (id, name, age, active, score, data) VALUES (2, 'Bob', 25, FALSE, 87.5, ?)")
    .bind(b"more_data".to_vec())
    .execute(&pool)
    .await
    .expect("Failed to insert");

  sqlx::query("INSERT INTO test_table (id, name, age, active, score, data) VALUES (3, 'Charlie', 35, TRUE, 92.0, NULL)")
    .execute(&pool)
    .await
    .expect("Failed to insert");

  pool.close().await;
  url
}

#[tokio::test]
#[ignore] // Requires MySQL database
async fn test_mysql_producer_new() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1");

  let producer = MysqlProducer::new(config.clone());
  assert_eq!(producer.db_config().connection_url, config.connection_url);
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_with_error_strategy() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1");

  let producer = MysqlProducer::new(config).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config().error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_with_name() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1");

  let producer = MysqlProducer::new(config).with_name("test_producer".to_string());
  assert_eq!(producer.config().name, Some("test_producer".to_string()));
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_clone() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1");

  let producer = MysqlProducer::new(config).with_name("test".to_string());
  let cloned = producer.clone();
  assert_eq!(producer.config().name, cloned.config().name);
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_simple_query() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT id, name, age FROM test_table ORDER BY id");

  let mut producer = MysqlProducer::new(config);
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
#[ignore]
async fn test_mysql_producer_produce_all_types() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM test_table WHERE id = 1");

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();

  assert_eq!(row.get("id"), Some(&Value::Number(1.into())));
  assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
  assert_eq!(row.get("age"), Some(&Value::Number(30.into())));
  assert_eq!(row.get("active"), Some(&Value::Bool(true)));
  assert_eq!(
    row.get("score"),
    Some(&Value::Number(serde_json::Number::from_f64(95.5).unwrap()))
  );
  // BLOB is base64 encoded
  assert!(row.get("data").is_some());
  let data = row.get("data").unwrap().as_str().unwrap();
  assert!(data.len() > 0);
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_with_parameters() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM test_table WHERE id = ? AND age > ?")
    .with_parameter(Value::Number(1.into()))
    .with_parameter(Value::Number(20.into()));

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 1);
  assert_eq!(rows[0].get("id"), Some(&Value::Number(1.into())));
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_with_string_parameter() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM test_table WHERE name = ?")
    .with_parameter(Value::String("Alice".to_string()));

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();

  assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_with_bool_parameter() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM test_table WHERE active = ?")
    .with_parameter(Value::Bool(true));

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 2);
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_with_null_parameter() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM test_table WHERE data IS ?")
    .with_parameter(Value::Null);

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let mut rows = Vec::new();

  while let Some(row) = stream.next().await {
    rows.push(row);
  }

  assert_eq!(rows.len(), 1);
  assert_eq!(rows[0].get("id"), Some(&Value::Number(3.into())));
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_with_json_parameter() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT ? as json_data")
    .with_parameter(Value::Object({
      let mut map = serde_json::Map::new();
      map.insert("key".to_string(), Value::String("value".to_string()));
      map
    }));

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await.unwrap();
  assert!(row.get("json_data").is_some());
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_produce_empty_result() {
  let url = setup_test_db().await;
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM test_table WHERE id = 999");

  let mut producer = MysqlProducer::new(config);
  let mut stream = producer.produce();
  let row = stream.next().await;
  assert!(row.is_none());
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_config_methods() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1")
    .with_max_connections(5)
    .with_fetch_size(500)
    .with_ssl(false);

  let producer = MysqlProducer::new(config.clone());
  let db_config = producer.db_config();

  assert_eq!(db_config.connection_url, config.connection_url);
  assert_eq!(db_config.database_type, config.database_type);
  assert_eq!(db_config.query, config.query);
  assert_eq!(db_config.max_connections, config.max_connections);
  assert_eq!(db_config.fetch_size, config.fetch_size);
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_set_config() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1");

  let mut producer = MysqlProducer::new(config);
  let new_config = streamweave::ProducerConfig::default();
  producer.set_config(new_config.clone());
  assert_eq!(producer.config(), &new_config);
}

#[tokio::test]
#[ignore]
async fn test_mysql_producer_get_config_mut() {
  let url = get_mysql_url();
  let config = DatabaseProducerConfig::default()
    .with_connection_url(&url)
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT 1");

  let mut producer = MysqlProducer::new(config);
  let config_mut = producer.get_config_mut();
  config_mut.name = Some("modified".to_string());
  assert_eq!(producer.config().name, Some("modified".to_string()));
}
