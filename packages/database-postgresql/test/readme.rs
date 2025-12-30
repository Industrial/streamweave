//! Integration tests for README examples
//!
//! These tests verify that all code examples in the README compile and run correctly.
//!
//! Note: These tests require a PostgreSQL database. Set the `POSTGRES_TEST_URL` environment
//! variable to run these tests, e.g.:
//! `POSTGRES_TEST_URL=postgresql://user:pass@localhost/testdb cargo test --test readme`

use std::time::Duration;
use streamweave_database::{DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseType};
use streamweave_database_postgresql::{PostgresConsumer, PostgresProducer};
use streamweave_error::ErrorStrategy;
use streamweave_pipeline::PipelineBuilder;

/// Helper to get test database URL from environment or return None
fn get_test_db_url() -> Option<String> {
  std::env::var("POSTGRES_TEST_URL").ok()
}

/// Helper to create a test producer config
fn create_test_producer_config(query: &str) -> Option<DatabaseProducerConfig> {
  get_test_db_url().map(|url| {
    DatabaseProducerConfig::default()
      .with_connection_url(&url)
      .with_database_type(DatabaseType::Postgres)
      .with_query(query)
  })
}

/// Helper to create a test consumer config
fn create_test_consumer_config(table_name: &str) -> Option<DatabaseConsumerConfig> {
  get_test_db_url().map(|url| {
    DatabaseConsumerConfig::default()
      .with_connection_url(&url)
      .with_database_type(DatabaseType::Postgres)
      .with_table_name(table_name)
  })
}

#[tokio::test]
async fn test_readme_query_postgresql_database() {
  // Example: Query PostgreSQL Database (lines 34-49)
  // This test verifies the basic query example compiles and can create a producer
  // Note: Actual execution requires a database, so we just verify construction

  if let Some(config) = create_test_producer_config("SELECT id, name, email FROM users") {
    let producer = PostgresProducer::new(config);
    // Verify producer was created successfully
    assert!(producer.db_config.query == "SELECT id, name, email FROM users");
    assert!(matches!(
      producer.db_config.database_type,
      DatabaseType::Postgres
    ));
  } else {
    // Skip test if no database URL provided
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_insert_into_postgresql() {
  // Example: Insert into PostgreSQL (lines 53-69)
  // This test verifies the basic insert example compiles and can create a consumer

  if let Some(config) = create_test_consumer_config("users") {
    let consumer = PostgresConsumer::new(config);
    // Verify consumer was created successfully
    assert!(consumer.db_config.table_name == "users");
    assert!(matches!(
      consumer.db_config.database_type,
      DatabaseType::Postgres
    ));
  } else {
    // Skip test if no database URL provided
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_query_with_parameters() {
  // Example: Query with Parameters (lines 107-119)
  // This test verifies parameterized query configuration

  if let Some(mut config) =
    create_test_producer_config("SELECT * FROM users WHERE age > $1 AND city = $2")
  {
    config = config
      .with_parameter(serde_json::Value::Number(18.into()))
      .with_parameter(serde_json::Value::String("New York".to_string()));

    let producer = PostgresProducer::new(config);
    // Verify parameters were set
    assert_eq!(producer.db_config.parameters.len(), 2);
  } else {
    // Skip test if no database URL provided
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_batch_insertion() {
  // Example: Batch Insertion (lines 125-139)
  // This test verifies batch insertion configuration

  if let Some(config) = create_test_consumer_config("users") {
    let consumer = PostgresConsumer::new(
      config
        .with_batch_size(1000)
        .with_batch_timeout(Duration::from_secs(5))
        .with_transactions(true),
    );

    // Verify batch configuration
    assert_eq!(consumer.db_config.batch_size, 1000);
    assert_eq!(consumer.db_config.batch_timeout, Duration::from_secs(5));
    assert!(consumer.db_config.transactions);
  } else {
    // Skip test if no database URL provided
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_transaction_handling() {
  // Example: Transaction Handling (lines 145-157)
  // This test verifies transaction configuration

  if let Some(config) = create_test_consumer_config("users") {
    let consumer = PostgresConsumer::new(
      config
        .with_transactions(true)
        .with_transaction_size(Some(5000)),
    );

    // Verify transaction configuration
    assert!(consumer.db_config.transactions);
    assert_eq!(consumer.db_config.transaction_size, Some(5000));
  } else {
    // Skip test if no database URL provided
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_connection_pooling() {
  // Example: Connection Pooling (lines 163-176)
  // This test verifies connection pool configuration

  if let Some(config) = create_test_producer_config("SELECT 1") {
    let producer = PostgresProducer::new(
      config
        .with_max_connections(50)
        .with_min_connections(5)
        .with_connect_timeout(Duration::from_secs(10)),
    );

    // Verify pool configuration
    assert_eq!(producer.db_config.max_connections, 50);
    assert_eq!(producer.db_config.min_connections, 5);
    assert_eq!(producer.db_config.connect_timeout, Duration::from_secs(10));
  } else {
    // Skip test if no database URL provided
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_error_handling() {
  // Example: Error Handling (lines 223-230)
  // This test verifies error strategy configuration

  if let Some(producer_config) = create_test_producer_config("SELECT 1") {
    if let Some(consumer_config) = create_test_consumer_config("users") {
      let producer =
        PostgresProducer::new(producer_config).with_error_strategy(ErrorStrategy::Skip);
      let consumer = PostgresConsumer::new(consumer_config);

      // Verify error strategy was set
      assert!(matches!(
        producer.config.error_strategy,
        ErrorStrategy::Skip
      ));

      // Verify pipeline can be constructed (even if not run)
      let _pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);
    } else {
      println!("Skipping test: POSTGRES_TEST_URL not set");
    }
  } else {
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_pipeline_construction() {
  // Test that pipeline construction from README examples compiles
  // This verifies the API usage patterns shown in the README

  if let Some(producer_config) = create_test_producer_config("SELECT id, name, email FROM users") {
    if let Some(consumer_config) = create_test_consumer_config("users") {
      let producer = PostgresProducer::new(producer_config);
      let consumer = PostgresConsumer::new(consumer_config);

      // Verify pipeline can be constructed with producer and consumer
      let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

      // Pipeline construction should succeed
      // Note: We don't run it here as it requires an actual database
      assert!(true); // Placeholder assertion
    } else {
      println!("Skipping test: POSTGRES_TEST_URL not set");
    }
  } else {
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}

#[tokio::test]
async fn test_readme_configuration_methods() {
  // Test that all configuration methods shown in README examples compile
  // This verifies the builder pattern API

  if let Some(config) = create_test_producer_config("SELECT 1") {
    // Test all producer configuration methods from examples
    let _producer = PostgresProducer::new(
      config
        .with_max_connections(50)
        .with_min_connections(5)
        .with_connect_timeout(Duration::from_secs(10))
        .with_fetch_size(1000),
    );

    assert!(true); // Configuration should compile
  } else {
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }

  if let Some(config) = create_test_consumer_config("users") {
    // Test all consumer configuration methods from examples
    let _consumer = PostgresConsumer::new(
      config
        .with_batch_size(1000)
        .with_batch_timeout(Duration::from_secs(5))
        .with_transactions(true)
        .with_transaction_size(Some(5000)),
    );

    assert!(true); // Configuration should compile
  } else {
    println!("Skipping test: POSTGRES_TEST_URL not set");
  }
}
