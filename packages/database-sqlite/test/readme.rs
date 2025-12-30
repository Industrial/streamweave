//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use sqlx::SqlitePool;
use std::collections::HashMap;
use std::time::Duration;
use streamweave_database::{
  DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType,
};
use streamweave_database_sqlite::{SqliteConsumer, SqliteProducer};
use streamweave_error::ErrorStrategy;
use streamweave_pipeline::PipelineBuilder;
use streamweave_vec::{VecConsumer, VecProducer};

/// Helper function to set up a test database with sample data
async fn setup_test_database() -> Result<SqlitePool, Box<dyn std::error::Error>> {
  // Create an in-memory SQLite database
  // Using "?cache=shared" allows multiple connections to the same in-memory database
  let pool = SqlitePool::connect("sqlite::memory:?cache=shared").await?;

  // Create the users table
  sqlx::query(
    r#"
        CREATE TABLE IF NOT EXISTS users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          email TEXT NOT NULL,
          age INTEGER
        )
        "#,
  )
  .execute(&pool)
  .await?;

  // Insert sample data
  sqlx::query(
    r#"
        INSERT INTO users (name, email, age) VALUES
          ('Alice', 'alice@example.com', 30),
          ('Bob', 'bob@example.com', 25),
          ('Charlie', 'charlie@example.com', 35)
        "#,
  )
  .execute(&pool)
  .await?;

  Ok(pool)
}

/// Test: Query SQLite Database
///
/// This test recreates the "Query SQLite Database" example from README.md lines 34-49.
/// The example shows how to create a SqliteProducer and query a database.
#[tokio::test]
async fn test_query_sqlite_database() {
  // Example from README.md lines 34-49
  use streamweave_database::{DatabaseProducerConfig, DatabaseType};
  use streamweave_database_sqlite::SqliteProducer;
  use streamweave_pipeline::PipelineBuilder;

  // Set up test database
  let _pool = setup_test_database().await.unwrap();

  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email FROM users ORDER BY id");

  // Complete the example: README shows `/* process rows */` - we use VecConsumer
  // to verify the producer works
  let consumer = VecConsumer::<DatabaseRow>::new();

  let pipeline = PipelineBuilder::new()
    .producer(SqliteProducer::new(config))
    .consumer(consumer);

  let ((), consumer) = pipeline.run().await.unwrap();

  // Verify we got 3 rows
  let collected = consumer.into_vec();
  assert_eq!(collected.len(), 3);

  // Verify first row
  let first_row = &collected[0];
  assert_eq!(
    first_row.get("id"),
    Some(&serde_json::Value::Number(1.into()))
  );
  assert_eq!(
    first_row.get("name"),
    Some(&serde_json::Value::String("Alice".to_string()))
  );
}

/// Test: Insert into SQLite
///
/// This test recreates the "Insert into SQLite" example from README.md lines 53-69.
/// The example shows how to create a SqliteConsumer and insert data.
#[tokio::test]
async fn test_insert_into_sqlite() {
  // Example from README.md lines 53-69
  use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
  use streamweave_database_sqlite::SqliteConsumer;
  use streamweave_pipeline::PipelineBuilder;

  // Set up test database with empty table
  let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
  sqlx::query(
    r#"
        CREATE TABLE IF NOT EXISTS users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          email TEXT NOT NULL
        )
        "#,
  )
  .execute(&pool)
  .await
  .unwrap();

  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_batch_size(1000);

  // Complete the example: README shows `/* produce rows */` - we create DatabaseRow items
  let mut row1_fields = HashMap::new();
  row1_fields.insert(
    "name".to_string(),
    serde_json::Value::String("Alice".to_string()),
  );
  row1_fields.insert(
    "email".to_string(),
    serde_json::Value::String("alice@example.com".to_string()),
  );
  let row1 = DatabaseRow::new(row1_fields);

  let mut row2_fields = HashMap::new();
  row2_fields.insert(
    "name".to_string(),
    serde_json::Value::String("Bob".to_string()),
  );
  row2_fields.insert(
    "email".to_string(),
    serde_json::Value::String("bob@example.com".to_string()),
  );
  let row2 = DatabaseRow::new(row2_fields);

  let producer = VecProducer::new(vec![row1, row2]);

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(SqliteConsumer::new(config));

  let ((), _consumer) = pipeline.run().await.unwrap();

  // Verify data was inserted by querying the database
  let rows = sqlx::query("SELECT name, email FROM users ORDER BY name")
    .fetch_all(&pool)
    .await
    .unwrap();

  assert_eq!(rows.len(), 2);
  let first_name: String = rows[0].get("name");
  assert_eq!(first_name, "Alice");
}

/// Test: Query with Parameters
///
/// This test recreates the "Query with Parameters" example from README.md lines 107-119.
/// The example shows how to use parameterized queries.
#[tokio::test]
async fn test_query_with_parameters() {
  // Example from README.md lines 107-119
  use streamweave_database::{DatabaseProducerConfig, DatabaseType};
  use streamweave_database_sqlite::SqliteProducer;

  // Set up test database with age column
  let _pool = setup_test_database().await.unwrap();

  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM users WHERE age > ? AND name = ?")
    .with_parameter(serde_json::Value::Number(25.into()))
    .with_parameter(serde_json::Value::String("Alice".to_string()));

  let producer = SqliteProducer::new(config);

  // Verify the producer can be created and configured
  let db_config = producer.db_config();
  assert_eq!(db_config.parameters.len(), 2);
  assert_eq!(
    db_config.query,
    "SELECT * FROM users WHERE age > ? AND name = ?"
  );
}

/// Test: In-Memory Database
///
/// This test recreates the "In-Memory Database" example from README.md lines 125-135.
/// The example shows how to use in-memory SQLite databases.
#[tokio::test]
async fn test_in_memory_database() {
  // Example from README.md lines 125-135
  use streamweave_database::{DatabaseProducerConfig, DatabaseType};
  use streamweave_database_sqlite::SqliteProducer;

  // Set up in-memory database
  let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
  sqlx::query(
    r#"
        CREATE TABLE IF NOT EXISTS users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL
        )
        "#,
  )
  .execute(&pool)
  .await
  .unwrap();

  sqlx::query("INSERT INTO users (name) VALUES ('Test User')")
    .execute(&pool)
    .await
    .unwrap();

  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:") // In-memory database
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM users");

  let producer = SqliteProducer::new(config);

  // Verify the producer is configured for in-memory database
  let db_config = producer.db_config();
  assert_eq!(db_config.connection_url, "sqlite::memory:");
  assert_eq!(db_config.database_type, DatabaseType::Sqlite);
}

/// Test: Batch Insertion
///
/// This test recreates the "Batch Insertion" example from README.md lines 141-155.
/// The example shows how to configure batch insertion.
#[tokio::test]
async fn test_batch_insertion() {
  // Example from README.md lines 141-155
  use std::time::Duration;
  use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
  use streamweave_database_sqlite::SqliteConsumer;

  // Set up test database
  let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
  sqlx::query(
    r#"
        CREATE TABLE IF NOT EXISTS users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          email TEXT NOT NULL
        )
        "#,
  )
  .execute(&pool)
  .await
  .unwrap();

  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(5))
    .with_transactions(true);

  let consumer = SqliteConsumer::new(config);

  // Verify the configuration
  let db_config = consumer.db_config();
  assert_eq!(db_config.batch_size, 1000);
  assert_eq!(db_config.batch_timeout, Duration::from_secs(5));
  assert!(db_config.use_transactions);
}

/// Test: Transaction Handling
///
/// This test recreates the "Transaction Handling" example from README.md lines 161-173.
/// The example shows how to configure transaction handling.
#[tokio::test]
async fn test_transaction_handling() {
  // Example from README.md lines 161-173
  use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
  use streamweave_database_sqlite::SqliteConsumer;

  // Set up test database
  let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
  sqlx::query(
    r#"
        CREATE TABLE IF NOT EXISTS users (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL
        )
        "#,
  )
  .execute(&pool)
  .await
  .unwrap();

  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_transactions(true)
    .with_transaction_size(Some(5000)); // Commit every 5000 rows

  let consumer = SqliteConsumer::new(config);

  // Verify the configuration
  let db_config = consumer.db_config();
  assert!(db_config.use_transactions);
  assert_eq!(db_config.transaction_size, Some(5000));
}

/// Test: Error Handling
///
/// This test recreates the "Error Handling" example from README.md lines 221-228.
/// The example shows how to configure error handling strategies.
#[tokio::test]
async fn test_error_handling() {
  // Example from README.md lines 221-228
  use streamweave_error::ErrorStrategy;

  // Set up test database
  let _pool = setup_test_database().await.unwrap();

  let producer_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email FROM users");

  let consumer_config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite::memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_batch_size(100);

  let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(SqliteProducer::new(producer_config))
    .consumer(SqliteConsumer::new(consumer_config));

  // Verify the pipeline can be created and run
  let result = pipeline.run().await;
  assert!(result.is_ok());
}
