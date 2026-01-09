//! Tests for DatabaseQuery node

use futures::StreamExt;
use futures::stream;
use streamweave::db::{DatabaseProducerConfig, DatabaseType};
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::DatabaseQuery;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_database_query_new() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite);
  let node = DatabaseQuery::new(config);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_database_query_with_name() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite);
  let node = DatabaseQuery::new(config).with_name("db_query_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("db_query_node"));
}

#[tokio::test]
async fn test_database_query_with_error_strategy() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite);
  let node = DatabaseQuery::new(config).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_database_query_config_access() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite);
  let mut node = DatabaseQuery::new(config).with_name("test".to_string());
  let config_ref = node.get_config_impl();
  assert_eq!(config_ref.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_database_query_clone() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite);
  let node1 = DatabaseQuery::new(config).with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}

#[tokio::test]
async fn test_database_query_different_db_types() {
  let sqlite_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite);

  let postgres_config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://localhost/test")
    .with_database_type(DatabaseType::Postgres);

  let mysql_config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://localhost/test")
    .with_database_type(DatabaseType::Mysql);

  let _sqlite_node = DatabaseQuery::new(sqlite_config);
  let _postgres_node = DatabaseQuery::new(postgres_config);
  let _mysql_node = DatabaseQuery::new(mysql_config);

  // Verify all database types can be configured
  assert!(true);
}

// Note: Full integration tests for database queries would require:
// - A test database setup
// - Actual database connections
// - Test data setup and cleanup
// - Query execution verification
// These are typically done in integration tests with testcontainers or similar
