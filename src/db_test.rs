use crate::db::*;
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// DatabaseType Tests
// ============================================================================

#[test]
fn test_database_type_default() {
  assert_eq!(DatabaseType::default(), DatabaseType::Postgres);
}

#[test]
fn test_database_type_all_variants() {
  let postgres = DatabaseType::Postgres;
  let mysql = DatabaseType::Mysql;
  let sqlite = DatabaseType::Sqlite;

  assert_eq!(postgres, DatabaseType::Postgres);
  assert_eq!(mysql, DatabaseType::Mysql);
  assert_eq!(sqlite, DatabaseType::Sqlite);
}

#[test]
fn test_database_type_clone() {
  let db_type = DatabaseType::Postgres;
  assert_eq!(db_type, db_type.clone());
}

#[test]
fn test_database_type_eq() {
  assert_eq!(DatabaseType::Postgres, DatabaseType::Postgres);
  assert_ne!(DatabaseType::Postgres, DatabaseType::Mysql);
  assert_ne!(DatabaseType::Mysql, DatabaseType::Sqlite);
}

#[test]
fn test_database_type_debug() {
  let db_type = DatabaseType::Postgres;
  let _ = format!("{:?}", db_type);
}

#[test]
fn test_database_type_serde() {
  let db_type = DatabaseType::Postgres;
  let serialized = serde_json::to_string(&db_type).unwrap();
  let deserialized: DatabaseType = serde_json::from_str(&serialized).unwrap();
  assert_eq!(db_type, deserialized);
}

#[test]
fn test_database_type_serde_all_variants() {
  for variant in [
    DatabaseType::Postgres,
    DatabaseType::Mysql,
    DatabaseType::Sqlite,
  ] {
    let serialized = serde_json::to_string(&variant).unwrap();
    let deserialized: DatabaseType = serde_json::from_str(&serialized).unwrap();
    assert_eq!(variant, deserialized);
  }
}

// ============================================================================
// DatabaseRow Tests
// ============================================================================

#[test]
fn test_database_row_new() {
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::json!(1));
  fields.insert("name".to_string(), serde_json::json!("test"));
  let row = DatabaseRow::new(fields.clone());

  assert_eq!(row.fields, fields);
}

#[test]
fn test_database_row_get() {
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::json!(42));
  fields.insert("name".to_string(), serde_json::json!("test"));
  let row = DatabaseRow::new(fields);

  assert_eq!(row.get("id"), Some(&serde_json::json!(42)));
  assert_eq!(row.get("name"), Some(&serde_json::json!("test")));
  assert_eq!(row.get("nonexistent"), None);
}

#[test]
fn test_database_row_get_different_types() {
  let mut fields = HashMap::new();
  fields.insert("int".to_string(), serde_json::json!(42));
  fields.insert("string".to_string(), serde_json::json!("hello"));
  fields.insert("bool".to_string(), serde_json::json!(true));
  fields.insert("null".to_string(), serde_json::json!(null));
  fields.insert("array".to_string(), serde_json::json!([1, 2, 3]));
  fields.insert("object".to_string(), serde_json::json!({"key": "value"}));
  let row = DatabaseRow::new(fields);

  assert_eq!(row.get("int"), Some(&serde_json::json!(42)));
  assert_eq!(row.get("string"), Some(&serde_json::json!("hello")));
  assert_eq!(row.get("bool"), Some(&serde_json::json!(true)));
  assert_eq!(row.get("null"), Some(&serde_json::json!(null)));
  assert_eq!(row.get("array"), Some(&serde_json::json!([1, 2, 3])));
  assert_eq!(
    row.get("object"),
    Some(&serde_json::json!({"key": "value"}))
  );
}

#[test]
fn test_database_row_columns() {
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::json!(1));
  fields.insert("name".to_string(), serde_json::json!("test"));
  fields.insert("email".to_string(), serde_json::json!("test@example.com"));
  let row = DatabaseRow::new(fields);

  let columns = row.columns();
  assert_eq!(columns.len(), 3);
  assert!(columns.contains(&"id".to_string()));
  assert!(columns.contains(&"name".to_string()));
  assert!(columns.contains(&"email".to_string()));
}

#[test]
fn test_database_row_columns_empty() {
  let fields = HashMap::new();
  let row = DatabaseRow::new(fields);
  assert!(row.columns().is_empty());
}

#[test]
fn test_database_row_clone() {
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::json!(1));
  let row1 = DatabaseRow::new(fields);
  let row2 = row1.clone();
  assert_eq!(row1, row2);
}

#[test]
fn test_database_row_eq() {
  let mut fields1 = HashMap::new();
  fields1.insert("id".to_string(), serde_json::json!(1));
  let row1 = DatabaseRow::new(fields1);

  let mut fields2 = HashMap::new();
  fields2.insert("id".to_string(), serde_json::json!(1));
  let row2 = DatabaseRow::new(fields2);

  assert_eq!(row1, row2);
}

#[test]
fn test_database_row_debug() {
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::json!(1));
  let row = DatabaseRow::new(fields);
  let _ = format!("{:?}", row);
}

#[test]
fn test_database_row_serde() {
  let mut fields = HashMap::new();
  fields.insert("id".to_string(), serde_json::json!(1));
  fields.insert("name".to_string(), serde_json::json!("test"));
  let row = DatabaseRow::new(fields);

  let serialized = serde_json::to_string(&row).unwrap();
  let deserialized: DatabaseRow = serde_json::from_str(&serialized).unwrap();
  assert_eq!(row, deserialized);
}

// ============================================================================
// DatabaseProducerConfig Tests
// ============================================================================

#[test]
fn test_database_producer_config_default() {
  let config = DatabaseProducerConfig::default();
  assert_eq!(config.connection_url, "");
  assert_eq!(config.database_type, DatabaseType::Postgres);
  assert_eq!(config.query, "");
  assert!(config.parameters.is_empty());
  assert_eq!(config.max_connections, 10);
  assert_eq!(config.min_connections, 2);
  assert_eq!(config.connect_timeout, Duration::from_secs(30));
  assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
  assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
  assert_eq!(config.fetch_size, 1000);
  assert!(!config.enable_ssl);
}

#[test]
fn test_database_producer_config_with_connection_url() {
  let config = DatabaseProducerConfig::default().with_connection_url("postgresql://localhost/test");
  assert_eq!(config.connection_url, "postgresql://localhost/test");
}

#[test]
fn test_database_producer_config_with_database_type() {
  let config = DatabaseProducerConfig::default().with_database_type(DatabaseType::Mysql);
  assert_eq!(config.database_type, DatabaseType::Mysql);

  let config = DatabaseProducerConfig::default().with_database_type(DatabaseType::Sqlite);
  assert_eq!(config.database_type, DatabaseType::Sqlite);
}

#[test]
fn test_database_producer_config_with_query() {
  let config = DatabaseProducerConfig::default().with_query("SELECT * FROM users");
  assert_eq!(config.query, "SELECT * FROM users");
}

#[test]
fn test_database_producer_config_with_parameter() {
  let config = DatabaseProducerConfig::default()
    .with_parameter(serde_json::json!(42))
    .with_parameter(serde_json::json!("test"));
  assert_eq!(config.parameters.len(), 2);
  assert_eq!(config.parameters[0], serde_json::json!(42));
  assert_eq!(config.parameters[1], serde_json::json!("test"));
}

#[test]
fn test_database_producer_config_with_parameters() {
  let params = vec![
    serde_json::json!(1),
    serde_json::json!(2),
    serde_json::json!(3),
  ];
  let config = DatabaseProducerConfig::default().with_parameters(params.clone());
  assert_eq!(config.parameters, params);
}

#[test]
fn test_database_producer_config_with_max_connections() {
  let config = DatabaseProducerConfig::default().with_max_connections(20);
  assert_eq!(config.max_connections, 20);
}

#[test]
fn test_database_producer_config_with_fetch_size() {
  let config = DatabaseProducerConfig::default().with_fetch_size(500);
  assert_eq!(config.fetch_size, 500);
}

#[test]
fn test_database_producer_config_with_ssl() {
  let config = DatabaseProducerConfig::default().with_ssl(true);
  assert!(config.enable_ssl);

  let config = DatabaseProducerConfig::default().with_ssl(false);
  assert!(!config.enable_ssl);
}

#[test]
fn test_database_producer_config_builder_chain() {
  let config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://localhost/test")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT * FROM users WHERE id = $1")
    .with_parameter(serde_json::json!(42))
    .with_max_connections(20)
    .with_fetch_size(500)
    .with_ssl(true);

  assert_eq!(config.connection_url, "postgresql://localhost/test");
  assert_eq!(config.database_type, DatabaseType::Postgres);
  assert_eq!(config.query, "SELECT * FROM users WHERE id = $1");
  assert_eq!(config.parameters.len(), 1);
  assert_eq!(config.max_connections, 20);
  assert_eq!(config.fetch_size, 500);
  assert!(config.enable_ssl);
}

#[test]
fn test_database_producer_config_clone() {
  let config1 = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://localhost/test")
    .with_query("SELECT * FROM users")
    .with_max_connections(20);
  let config2 = config1.clone();
  assert_eq!(config1.connection_url, config2.connection_url);
  assert_eq!(config1.query, config2.query);
  assert_eq!(config1.max_connections, config2.max_connections);
}

#[test]
fn test_database_producer_config_debug() {
  let config = DatabaseProducerConfig::default();
  let _ = format!("{:?}", config);
}

// ============================================================================
// DatabaseConsumerConfig Tests
// ============================================================================

#[test]
fn test_database_consumer_config_default() {
  let config = DatabaseConsumerConfig::default();
  assert_eq!(config.connection_url, "");
  assert_eq!(config.database_type, DatabaseType::Postgres);
  assert_eq!(config.table_name, "");
  assert_eq!(config.insert_query, None);
  assert_eq!(config.batch_size, 100);
  assert_eq!(config.batch_timeout, Duration::from_secs(5));
  assert_eq!(config.column_mapping, None);
  assert!(config.use_transactions);
  assert_eq!(config.transaction_size, None);
  assert_eq!(config.max_connections, 10);
  assert_eq!(config.min_connections, 2);
  assert_eq!(config.connect_timeout, Duration::from_secs(30));
  assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
  assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
  assert!(!config.enable_ssl);
  assert_eq!(config.name, "");
}

#[test]
fn test_database_consumer_config_with_connection_url() {
  let config = DatabaseConsumerConfig::default().with_connection_url("postgresql://localhost/test");
  assert_eq!(config.connection_url, "postgresql://localhost/test");
}

#[test]
fn test_database_consumer_config_with_database_type() {
  let config = DatabaseConsumerConfig::default().with_database_type(DatabaseType::Mysql);
  assert_eq!(config.database_type, DatabaseType::Mysql);

  let config = DatabaseConsumerConfig::default().with_database_type(DatabaseType::Sqlite);
  assert_eq!(config.database_type, DatabaseType::Sqlite);
}

#[test]
fn test_database_consumer_config_with_table_name() {
  let config = DatabaseConsumerConfig::default().with_table_name("users");
  assert_eq!(config.table_name, "users");
}

#[test]
fn test_database_consumer_config_with_insert_query() {
  let config = DatabaseConsumerConfig::default()
    .with_insert_query("INSERT INTO users (id, name) VALUES ($1, $2)");
  assert_eq!(
    config.insert_query,
    Some("INSERT INTO users (id, name) VALUES ($1, $2)".to_string())
  );
}

#[test]
fn test_database_consumer_config_with_batch_size() {
  let config = DatabaseConsumerConfig::default().with_batch_size(200);
  assert_eq!(config.batch_size, 200);
}

#[test]
fn test_database_consumer_config_with_batch_timeout() {
  let timeout = Duration::from_secs(10);
  let config = DatabaseConsumerConfig::default().with_batch_timeout(timeout);
  assert_eq!(config.batch_timeout, timeout);
}

#[test]
fn test_database_consumer_config_with_column_mapping() {
  let mut mapping = HashMap::new();
  mapping.insert("field_name".to_string(), "column_name".to_string());
  mapping.insert("email_addr".to_string(), "email".to_string());
  let config = DatabaseConsumerConfig::default().with_column_mapping(mapping.clone());
  assert_eq!(config.column_mapping, Some(mapping));
}

#[test]
fn test_database_consumer_config_with_transactions() {
  let config = DatabaseConsumerConfig::default().with_transactions(true);
  assert!(config.use_transactions);

  let config = DatabaseConsumerConfig::default().with_transactions(false);
  assert!(!config.use_transactions);
}

#[test]
fn test_database_consumer_config_with_transaction_size() {
  let config = DatabaseConsumerConfig::default().with_transaction_size(Some(50));
  assert_eq!(config.transaction_size, Some(50));

  let config = DatabaseConsumerConfig::default().with_transaction_size(None);
  assert_eq!(config.transaction_size, None);
}

#[test]
fn test_database_consumer_config_with_max_connections() {
  let config = DatabaseConsumerConfig::default().with_max_connections(20);
  assert_eq!(config.max_connections, 20);
}

#[test]
fn test_database_consumer_config_with_ssl() {
  let config = DatabaseConsumerConfig::default().with_ssl(true);
  assert!(config.enable_ssl);

  let config = DatabaseConsumerConfig::default().with_ssl(false);
  assert!(!config.enable_ssl);
}

#[test]
fn test_database_consumer_config_with_name() {
  let config = DatabaseConsumerConfig::default().with_name("my_consumer".to_string());
  assert_eq!(config.name, "my_consumer");
}

#[test]
fn test_database_consumer_config_builder_chain() {
  let mut mapping = HashMap::new();
  mapping.insert("field".to_string(), "column".to_string());

  let config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://localhost/test")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users")
    .with_insert_query("INSERT INTO users VALUES ($1, $2)")
    .with_batch_size(200)
    .with_batch_timeout(Duration::from_secs(10))
    .with_column_mapping(mapping.clone())
    .with_transactions(true)
    .with_transaction_size(Some(50))
    .with_max_connections(20)
    .with_ssl(true)
    .with_name("my_consumer".to_string());

  assert_eq!(config.connection_url, "postgresql://localhost/test");
  assert_eq!(config.database_type, DatabaseType::Postgres);
  assert_eq!(config.table_name, "users");
  assert_eq!(
    config.insert_query,
    Some("INSERT INTO users VALUES ($1, $2)".to_string())
  );
  assert_eq!(config.batch_size, 200);
  assert_eq!(config.batch_timeout, Duration::from_secs(10));
  assert_eq!(config.column_mapping, Some(mapping));
  assert!(config.use_transactions);
  assert_eq!(config.transaction_size, Some(50));
  assert_eq!(config.max_connections, 20);
  assert!(config.enable_ssl);
  assert_eq!(config.name, "my_consumer");
}

#[test]
fn test_database_consumer_config_clone() {
  let config1 = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://localhost/test")
    .with_table_name("users")
    .with_batch_size(200)
    .with_name("my_consumer".to_string());
  let config2 = config1.clone();
  assert_eq!(config1.connection_url, config2.connection_url);
  assert_eq!(config1.table_name, config2.table_name);
  assert_eq!(config1.batch_size, config2.batch_size);
  assert_eq!(config1.name, config2.name);
}

#[test]
fn test_database_consumer_config_debug() {
  let config = DatabaseConsumerConfig::default();
  let _ = format!("{:?}", config);
}
