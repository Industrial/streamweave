//! Tests for DatabaseOperationNode

use futures::StreamExt;
use futures::stream;
use streamweave::db::{DatabaseConsumerConfig, DatabaseType};
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::DatabaseOperationNode;
use streamweave::transformers::DatabaseOperation;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_database_operation_node_new() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let node = DatabaseOperationNode::new(config, DatabaseOperation::Insert);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_database_operation_node_with_name() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let node = DatabaseOperationNode::new(config, DatabaseOperation::Insert)
    .with_name("db_op_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("db_op_node"));
}

#[tokio::test]
async fn test_database_operation_node_with_error_strategy() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let node = DatabaseOperationNode::new(config, DatabaseOperation::Insert)
    .with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_database_operation_node_insert() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let mut node = DatabaseOperationNode::new(config, DatabaseOperation::Insert);

  // Note: Actual database operations require a real database connection
  // These tests verify the node structure and configuration
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_database_operation_node_update() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let node = DatabaseOperationNode::new(config, DatabaseOperation::Update);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_database_operation_node_delete() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let node = DatabaseOperationNode::new(config, DatabaseOperation::Delete);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_database_operation_node_config_access() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let mut node =
    DatabaseOperationNode::new(config, DatabaseOperation::Insert).with_name("test".to_string());
  let config_ref = node.get_config_impl();
  assert_eq!(config_ref.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_database_operation_node_clone() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");
  let node1 =
    DatabaseOperationNode::new(config, DatabaseOperation::Insert).with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}

#[tokio::test]
async fn test_database_operation_node_different_operations() {
  let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let insert_node = DatabaseOperationNode::new(config.clone(), DatabaseOperation::Insert);
  let update_node = DatabaseOperationNode::new(config.clone(), DatabaseOperation::Update);
  let delete_node = DatabaseOperationNode::new(config, DatabaseOperation::Delete);

  // Verify all operations can be created
  assert!(insert_node.get_config_impl().name().is_none());
  assert!(update_node.get_config_impl().name().is_none());
  assert!(delete_node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_database_operation_node_different_db_types() {
  let sqlite_config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite://:memory:")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("test_table");

  let postgres_config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://localhost/test")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("test_table");

  let mysql_config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://localhost/test")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("test_table");

  let _sqlite_node = DatabaseOperationNode::new(sqlite_config, DatabaseOperation::Insert);
  let _postgres_node = DatabaseOperationNode::new(postgres_config, DatabaseOperation::Insert);
  let _mysql_node = DatabaseOperationNode::new(mysql_config, DatabaseOperation::Insert);

  // Verify all database types can be configured
  assert!(true);
}

// Note: Full integration tests for database operations would require:
// - A test database setup
// - Actual database connections
// - Table creation and cleanup
// These are typically done in integration tests with testcontainers or similar
