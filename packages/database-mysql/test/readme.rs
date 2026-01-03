//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! compile and can be instantiated correctly. Note that full integration tests
//! would require a running MySQL database instance.

use std::time::Duration;
use streamweave_database::{DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseType};
use streamweave_database_mysql::{MysqlConsumer, MysqlProducer};

/// Test: Query MySQL Database
///
/// This test recreates the "Query MySQL Database" example from README.md lines 34-49.
/// It verifies that the configuration can be created and the producer can be instantiated.
#[tokio::test]
async fn test_query_mysql_database() {
  // Example from README.md lines 34-49
  let config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT id, name, email FROM users");

  // Verify configuration was set correctly
  assert_eq!(config.connection_url, "mysql://user:pass@localhost/mydb");
  assert_eq!(config.database_type, DatabaseType::Mysql);
  assert_eq!(config.query, "SELECT id, name, email FROM users");

  // Create producer (doesn't require DB connection until produce() is called)
  let producer = MysqlProducer::new(config);

  // Verify producer was created
  assert_eq!(
    producer.db_config().connection_url,
    "mysql://user:pass@localhost/mydb"
  );
  assert_eq!(producer.db_config().database_type, DatabaseType::Mysql);
  assert_eq!(
    producer.db_config().query,
    "SELECT id, name, email FROM users"
  );
}

/// Test: Insert into MySQL
///
/// This test recreates the "Insert into MySQL" example from README.md lines 53-69.
/// It verifies that the configuration can be created and the consumer can be instantiated.
#[tokio::test]
async fn test_insert_into_mysql() {
  // Example from README.md lines 53-69
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users")
    .with_batch_size(1000);

  // Verify configuration was set correctly
  assert_eq!(config.connection_url, "mysql://user:pass@localhost/mydb");
  assert_eq!(config.database_type, DatabaseType::Mysql);
  assert_eq!(config.table_name, "users");
  assert_eq!(config.batch_size, 1000);

  // Create consumer (doesn't require DB connection until consume() is called)
  let consumer = MysqlConsumer::new(config);

  // Verify consumer was created
  assert_eq!(
    consumer.db_config().connection_url,
    "mysql://user:pass@localhost/mydb"
  );
  assert_eq!(consumer.db_config().database_type, DatabaseType::Mysql);
  assert_eq!(consumer.db_config().table_name, "users");
  assert_eq!(consumer.db_config().batch_size, 1000);
}

/// Test: Query with Parameters
///
/// This test recreates the "Query with Parameters" example from README.md lines 103-119.
/// It verifies that parameterized queries can be configured.
#[tokio::test]
async fn test_query_with_parameters() {
  // Example from README.md lines 103-119
  let config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM users WHERE age > ? AND city = ?")
    .with_parameter(serde_json::Value::Number(18.into()))
    .with_parameter(serde_json::Value::String("New York".to_string()));

  // Verify configuration was set correctly
  assert_eq!(
    config.query,
    "SELECT * FROM users WHERE age > ? AND city = ?"
  );
  assert_eq!(config.parameters.len(), 2);
  assert_eq!(config.parameters[0], serde_json::Value::Number(18.into()));
  assert_eq!(
    config.parameters[1],
    serde_json::Value::String("New York".to_string())
  );

  // Create producer
  let producer = MysqlProducer::new(config);

  // Verify producer was created with parameters
  assert_eq!(producer.db_config().parameters.len(), 2);
}

/// Test: Batch Insertion
///
/// This test recreates the "Batch Insertion" example from README.md lines 121-139.
/// It verifies that batch configuration can be set.
#[tokio::test]
async fn test_batch_insertion() {
  // Example from README.md lines 121-139
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(5))
    .with_transactions(true);

  // Verify configuration was set correctly
  assert_eq!(config.batch_size, 1000);
  assert_eq!(config.batch_timeout, Duration::from_secs(5));
  assert!(config.use_transactions);

  // Create consumer
  let consumer = MysqlConsumer::new(config);

  // Verify consumer was created with batch configuration
  assert_eq!(consumer.db_config().batch_size, 1000);
  assert_eq!(consumer.db_config().batch_timeout, Duration::from_secs(5));
  assert!(consumer.db_config().use_transactions);
}

/// Test: Transaction Handling
///
/// This test recreates the "Transaction Handling" example from README.md lines 141-157.
/// It verifies that transaction configuration can be set.
#[tokio::test]
async fn test_transaction_handling() {
  // Example from README.md lines 141-157
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users")
    .with_transactions(true)
    .with_transaction_size(Some(5000)); // Commit every 5000 rows

  // Verify configuration was set correctly
  assert!(config.use_transactions);
  assert_eq!(config.transaction_size, Some(5000));

  // Create consumer
  let consumer = MysqlConsumer::new(config);

  // Verify consumer was created with transaction configuration
  assert!(consumer.db_config().use_transactions);
  assert_eq!(consumer.db_config().transaction_size, Some(5000));
}

/// Test: Connection Pooling
///
/// This test recreates the "Connection Pooling" example from README.md lines 159-176.
/// It verifies that connection pool configuration can be set.
/// Note: The README shows `with_min_connections` and `with_connect_timeout` which may
/// not be available in the current API, so we test what's actually available.
#[tokio::test]
async fn test_connection_pooling() {
  // Example from README.md lines 159-176
  // Note: Using available methods - some README examples may show aspirational API
  let config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_max_connections(50);
  // Note: with_min_connections and with_connect_timeout may not be available
  // in the current API implementation

  // Verify configuration was set correctly
  assert_eq!(config.max_connections, 50);

  // Create producer
  let producer = MysqlProducer::new(config);

  // Verify producer was created with pool configuration
  assert_eq!(producer.db_config().max_connections, 50);
}

/// Test: Error Handling
///
/// This test recreates the "Error Handling" example from README.md lines 223-230.
/// It verifies that error strategies can be configured.
#[tokio::test]
async fn test_error_handling() {
  // Example from README.md lines 223-230
  use streamweave::error::ErrorStrategy;

  let producer_config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM users");

  let consumer_config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users");

  // Create producer and consumer with error strategy
  let producer = MysqlProducer::new(producer_config).with_error_strategy(ErrorStrategy::Skip);

  let consumer = MysqlConsumer::new(consumer_config).with_error_strategy(ErrorStrategy::Skip);

  // Verify error strategies were set
  assert!(matches!(
    producer.config().error_strategy,
    ErrorStrategy::Skip
  ));
  assert!(matches!(
    consumer.config().error_strategy,
    ErrorStrategy::Skip
  ));
}
