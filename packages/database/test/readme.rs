//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use std::collections::HashMap;
use std::time::Duration;
use streamweave_database::{
  DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType,
};

/// Test: Using Database Types
///
/// This test recreates the "Using Database Types" example from README.md lines 34-48.
#[test]
fn test_using_database_types() {
  // Example from README.md lines 34-48
  use streamweave_database::{
    DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType,
  };

  // Configure producer
  let producer_config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/dbname")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT * FROM users");

  // Configure consumer
  let consumer_config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/dbname")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users");

  // Verify configurations were created correctly
  assert_eq!(
    producer_config.connection_url,
    "postgresql://user:pass@localhost/dbname"
  );
  assert_eq!(producer_config.database_type, DatabaseType::Postgres);
  assert_eq!(producer_config.query, "SELECT * FROM users");

  assert_eq!(
    consumer_config.connection_url,
    "postgresql://user:pass@localhost/dbname"
  );
  assert_eq!(consumer_config.database_type, DatabaseType::Postgres);
  assert_eq!(consumer_config.table_name, "users");
}

/// Test: Producer Configuration
///
/// This test recreates the "Producer Configuration" example from README.md lines 116-128.
#[test]
fn test_producer_configuration() {
  // Example from README.md lines 116-128
  use std::time::Duration;
  use streamweave_database::{DatabaseProducerConfig, DatabaseType};

  let config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT id, name, email FROM users WHERE active = $1")
    .with_parameter(serde_json::Value::Bool(true))
    .with_max_connections(20)
    .with_fetch_size(500)
    .with_ssl(true);

  // Verify configuration
  assert_eq!(
    config.connection_url,
    "postgresql://user:pass@localhost/mydb"
  );
  assert_eq!(config.database_type, DatabaseType::Postgres);
  assert_eq!(
    config.query,
    "SELECT id, name, email FROM users WHERE active = $1"
  );
  assert_eq!(config.parameters.len(), 1);
  assert_eq!(config.parameters[0], serde_json::Value::Bool(true));
  assert_eq!(config.max_connections, 20);
  assert_eq!(config.fetch_size, 500);
  assert!(config.enable_ssl);
}

/// Test: Consumer Configuration
///
/// This test recreates the "Consumer Configuration" example from README.md lines 134-152.
#[test]
fn test_consumer_configuration() {
  // Example from README.md lines 134-152
  use std::collections::HashMap;
  use std::time::Duration;
  use streamweave_database::{DatabaseConsumerConfig, DatabaseType};

  let mut column_mapping = HashMap::new();
  column_mapping.insert("user_id".to_string(), "id".to_string());
  column_mapping.insert("user_name".to_string(), "name".to_string());

  let config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(10))
    .with_column_mapping(column_mapping.clone())
    .with_transactions(true)
    .with_transaction_size(Some(500));

  // Verify configuration
  assert_eq!(
    config.connection_url,
    "postgresql://user:pass@localhost/mydb"
  );
  assert_eq!(config.database_type, DatabaseType::Postgres);
  assert_eq!(config.table_name, "users");
  assert_eq!(config.batch_size, 1000);
  assert_eq!(config.batch_timeout, Duration::from_secs(10));
  assert_eq!(config.column_mapping, column_mapping);
  assert!(config.use_transactions);
  assert_eq!(config.transaction_size, Some(500));
}

/// Test: Working with DatabaseRow
///
/// This test recreates the "Working with DatabaseRow" example from README.md lines 158-174.
#[test]
fn test_working_with_database_row() {
  // Example from README.md lines 158-174
  use std::collections::HashMap;
  use streamweave_database::DatabaseRow;

  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
  fields.insert(
    "name".to_string(),
    serde_json::Value::String("Alice".to_string()),
  );

  let row = DatabaseRow::new(fields);

  // Get value by column name
  let id = row.get("id");
  let name = row.get("name");

  // Get all column names
  let columns = row.columns();

  // Verify results
  assert_eq!(id, Some(&serde_json::Value::Number(1.into())));
  assert_eq!(name, Some(&serde_json::Value::String("Alice".to_string())));
  assert_eq!(columns.len(), 2);
  assert!(columns.contains(&"id".to_string()));
  assert!(columns.contains(&"name".to_string()));
}

/// Test: Parameterized Queries
///
/// This test recreates the "Parameterized Queries" example from README.md lines 180-187.
#[test]
fn test_parameterized_queries() {
  // Example from README.md lines 180-187
  use streamweave_database::DatabaseProducerConfig;

  let config = DatabaseProducerConfig::default()
    .with_query("SELECT * FROM users WHERE age > $1 AND city = $2")
    .with_parameter(serde_json::Value::Number(18.into()))
    .with_parameter(serde_json::Value::String("New York".to_string()));

  // Verify configuration
  assert_eq!(
    config.query,
    "SELECT * FROM users WHERE age > $1 AND city = $2"
  );
  assert_eq!(config.parameters.len(), 2);
  assert_eq!(config.parameters[0], serde_json::Value::Number(18.into()));
  assert_eq!(
    config.parameters[1],
    serde_json::Value::String("New York".to_string())
  );
}

/// Test: Connection Pooling
///
/// This test recreates the "Connection Pooling" example from README.md lines 193-203.
#[test]
fn test_connection_pooling() {
  // Example from README.md lines 193-203
  use std::time::Duration;
  use streamweave_database::DatabaseProducerConfig;

  let config = DatabaseProducerConfig::default()
    .with_max_connections(50)
    .with_min_connections(5)
    .with_connect_timeout(Duration::from_secs(10))
    .with_idle_timeout(Some(Duration::from_secs(300)))
    .with_max_lifetime(Some(Duration::from_secs(3600)));

  // Verify configuration
  assert_eq!(config.max_connections, 50);
  assert_eq!(config.min_connections, 5);
  assert_eq!(config.connect_timeout, Duration::from_secs(10));
  assert_eq!(config.idle_timeout, Some(Duration::from_secs(300)));
  assert_eq!(config.max_lifetime, Some(Duration::from_secs(3600)));
}

/// Test: Batch Insertion
///
/// This test recreates the "Batch Insertion" example from README.md lines 209-218.
#[test]
fn test_batch_insertion() {
  // Example from README.md lines 209-218
  use std::time::Duration;
  use streamweave_database::DatabaseConsumerConfig;

  let config = DatabaseConsumerConfig::default()
    .with_batch_size(1000) // Insert 1000 rows at a time
    .with_batch_timeout(Duration::from_secs(5)) // Flush after 5 seconds
    .with_transactions(true)
    .with_transaction_size(Some(5000)); // Commit every 5000 rows

  // Verify configuration
  assert_eq!(config.batch_size, 1000);
  assert_eq!(config.batch_timeout, Duration::from_secs(5));
  assert!(config.use_transactions);
  assert_eq!(config.transaction_size, Some(5000));
}

/// Test: DatabaseType Enum
///
/// This test verifies the DatabaseType enum from README.md lines 56-62.
#[test]
fn test_database_type_enum() {
  // Example from README.md lines 56-62
  use streamweave_database::DatabaseType;

  // Test all variants
  let postgres = DatabaseType::Postgres;
  let mysql = DatabaseType::Mysql;
  let sqlite = DatabaseType::Sqlite;

  // Verify they are different
  assert_ne!(postgres, mysql);
  assert_ne!(postgres, sqlite);
  assert_ne!(mysql, sqlite);

  // Test default
  assert_eq!(DatabaseType::default(), DatabaseType::Postgres);
}
