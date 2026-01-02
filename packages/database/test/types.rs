//! Property-based tests for database types using proptest.
//!
//! These tests use property-based testing to verify that the database types
//! maintain their invariants across a wide range of inputs.

use proptest::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use streamweave_database::{
  DatabaseConsumerConfig, DatabaseProducerConfig, DatabaseRow, DatabaseType,
};

/// Strategy for generating serde_json::Value
fn json_value_strategy() -> impl Strategy<Value = Value> {
  prop_oneof![
    Just(Value::Null),
    any::<bool>().prop_map(Value::Bool),
    any::<i64>().prop_map(|n| Value::Number(n.into())),
    any::<f64>().prop_map(|n| {
      // Ensure we don't create NaN or Infinity
      if n.is_finite() {
        Value::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
      } else {
        Value::Number(0.into())
      }
    }),
    prop::string::string_regex(".*")
      .unwrap()
      .prop_map(Value::String),
    prop::collection::vec(json_value_strategy(), 0..10).prop_map(Value::Array),
    prop::collection::hash_map(
      prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      json_value_strategy(),
      0..10
    )
    .prop_map(Value::Object),
  ]
  .boxed()
}

/// Strategy for generating DatabaseType
fn database_type_strategy() -> impl Strategy<Value = DatabaseType> {
  prop::sample::select(vec![
    DatabaseType::Postgres,
    DatabaseType::Mysql,
    DatabaseType::Sqlite,
  ])
}

/// Strategy for generating column names
fn column_name_strategy() -> impl Strategy<Value = String> {
  prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]*").unwrap()
}

/// Strategy for generating DatabaseRow fields
fn database_row_fields_strategy() -> impl Strategy<Value = HashMap<String, Value>> {
  prop::collection::hash_map(column_name_strategy(), json_value_strategy(), 0..20)
}

proptest! {
  // DatabaseType tests
  #[test]
  fn test_database_type_serialize_roundtrip(db_type in database_type_strategy()) {
    let json = serde_json::to_string(&db_type).unwrap();
    let deserialized: DatabaseType = serde_json::from_str(&json).unwrap();
    prop_assert_eq!(db_type, deserialized);
  }

  #[test]
  fn test_database_type_default_is_postgres(_ in any::<u8>()) {
    prop_assert_eq!(DatabaseType::default(), DatabaseType::Postgres);
  }

  #[test]
  fn test_database_type_equality(
    db_type1 in database_type_strategy(),
    db_type2 in database_type_strategy(),
  ) {
    if db_type1 == db_type2 {
      prop_assert_eq!(db_type1, db_type2);
    } else {
      prop_assert_ne!(db_type1, db_type2);
    }
  }

  // DatabaseRow tests
  #[test]
  fn test_database_row_new(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());
    prop_assert_eq!(row.fields.len(), fields.len());
  }

  #[test]
  fn test_database_row_get(
    fields in database_row_fields_strategy(),
    column_name in column_name_strategy(),
  ) {
    let row = DatabaseRow::new(fields.clone());
    let expected = fields.get(&column_name);
    prop_assert_eq!(row.get(&column_name), expected);
  }

  #[test]
  fn test_database_row_columns(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());
    let columns = row.columns();

    prop_assert_eq!(columns.len(), fields.len());

    // All columns should be present
    for col in &columns {
      prop_assert!(fields.contains_key(col));
    }

    // All field keys should be in columns
    for key in fields.keys() {
      prop_assert!(columns.contains(key));
    }
  }

  #[test]
  fn test_database_row_columns_contains_all_keys(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());
    let columns = row.columns();

    // Every key in fields should appear in columns
    for key in fields.keys() {
      prop_assert!(columns.contains(key));
    }
  }

  #[test]
  fn test_database_row_get_returns_correct_value(
    key in column_name_strategy(),
    value in json_value_strategy(),
  ) {
    let mut fields = HashMap::new();
    fields.insert(key.clone(), value.clone());
    let row = DatabaseRow::new(fields);

    prop_assert_eq!(row.get(&key), Some(&value));
  }

  #[test]
  fn test_database_row_get_nonexistent(
    fields in database_row_fields_strategy(),
    nonexistent_key in column_name_strategy(),
  ) {
    let row = DatabaseRow::new(fields.clone());
    if !fields.contains_key(&nonexistent_key) {
      prop_assert_eq!(row.get(&nonexistent_key), None);
    }
  }

  #[test]
  fn test_database_row_serialize_roundtrip(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());
    let json = serde_json::to_string(&row).unwrap();
    let deserialized: DatabaseRow = serde_json::from_str(&json).unwrap();

    prop_assert_eq!(row.fields.len(), deserialized.fields.len());
    for (key, value) in &row.fields {
      prop_assert_eq!(deserialized.get(key), Some(value));
    }
  }

  #[test]
  fn test_database_row_clone(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());
    let cloned = row.clone();

    prop_assert_eq!(row.fields.len(), cloned.fields.len());
    for (key, value) in &row.fields {
      prop_assert_eq!(cloned.get(key), Some(value));
    }
  }

  // DatabaseProducerConfig tests
  #[test]
  fn test_producer_config_with_connection_url(
    url in prop::string::string_regex(".*").unwrap()
  ) {
    let config = DatabaseProducerConfig::default()
      .with_connection_url(url.clone());
    prop_assert_eq!(config.connection_url, url);
  }

  #[test]
  fn test_producer_config_with_database_type(
    db_type in database_type_strategy()
  ) {
    let config = DatabaseProducerConfig::default()
      .with_database_type(db_type);
    prop_assert_eq!(config.database_type, db_type);
  }

  #[test]
  fn test_producer_config_with_query(
    query in prop::string::string_regex(".*").unwrap()
  ) {
    let config = DatabaseProducerConfig::default()
      .with_query(query.clone());
    prop_assert_eq!(config.query, query);
  }

  #[test]
  fn test_producer_config_with_parameter(
    param in json_value_strategy()
  ) {
    let config = DatabaseProducerConfig::default()
      .with_parameter(param.clone());
    prop_assert_eq!(config.parameters.len(), 1);
    prop_assert_eq!(config.parameters[0], param);
  }

  #[test]
  fn test_producer_config_with_parameters(
    params in prop::collection::vec(json_value_strategy(), 0..20)
  ) {
    let config = DatabaseProducerConfig::default()
      .with_parameters(params.clone());
    prop_assert_eq!(config.parameters, params);
  }

  #[test]
  fn test_producer_config_with_max_connections(
    max_conns in 1u32..1000u32
  ) {
    let config = DatabaseProducerConfig::default()
      .with_max_connections(max_conns);
    prop_assert_eq!(config.max_connections, max_conns);
  }

  #[test]
  fn test_producer_config_with_fetch_size(
    fetch_size in 1usize..10000usize
  ) {
    let config = DatabaseProducerConfig::default()
      .with_fetch_size(fetch_size);
    prop_assert_eq!(config.fetch_size, fetch_size);
  }

  #[test]
  fn test_producer_config_with_ssl(
    enable_ssl in any::<bool>()
  ) {
    let config = DatabaseProducerConfig::default()
      .with_ssl(enable_ssl);
    prop_assert_eq!(config.enable_ssl, enable_ssl);
  }

  #[test]
  fn test_producer_config_builder_chaining(
    url in prop::string::string_regex(".*").unwrap(),
    db_type in database_type_strategy(),
    query in prop::string::string_regex(".*").unwrap(),
    max_conns in 1u32..1000u32,
    fetch_size in 1usize..10000usize,
    enable_ssl in any::<bool>(),
  ) {
    let config = DatabaseProducerConfig::default()
      .with_connection_url(url.clone())
      .with_database_type(db_type)
      .with_query(query.clone())
      .with_max_connections(max_conns)
      .with_fetch_size(fetch_size)
      .with_ssl(enable_ssl);

    prop_assert_eq!(config.connection_url, url);
    prop_assert_eq!(config.query, query);
    prop_assert_eq!(config.max_connections, max_conns);
    prop_assert_eq!(config.fetch_size, fetch_size);
    prop_assert_eq!(config.enable_ssl, enable_ssl);
  }

  #[test]
  fn test_producer_config_clone(
    url in prop::string::string_regex(".*").unwrap(),
    query in prop::string::string_regex(".*").unwrap(),
  ) {
    let config = DatabaseProducerConfig::default()
      .with_connection_url(url.clone())
      .with_query(query.clone());
    let cloned = config.clone();

    prop_assert_eq!(config.connection_url, cloned.connection_url);
    prop_assert_eq!(config.query, cloned.query);
    prop_assert_eq!(config.database_type, cloned.database_type);
  }

  #[test]
  fn test_producer_config_default_values(_ in any::<u8>()) {
    let config = DatabaseProducerConfig::default();
    prop_assert_eq!(config.database_type, DatabaseType::Postgres);
    prop_assert_eq!(config.max_connections, 10);
    prop_assert_eq!(config.min_connections, 2);
    prop_assert_eq!(config.fetch_size, 1000);
    prop_assert!(!config.enable_ssl);
    prop_assert_eq!(config.connection_url, "");
    prop_assert_eq!(config.query, "");
    prop_assert_eq!(config.parameters.len(), 0);
  }

  // DatabaseConsumerConfig tests
  #[test]
  fn test_consumer_config_with_connection_url(
    url in prop::string::string_regex(".*").unwrap()
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_connection_url(url.clone());
    prop_assert_eq!(config.connection_url, url);
  }

  #[test]
  fn test_consumer_config_with_database_type(
    db_type in database_type_strategy()
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_database_type(db_type);
    prop_assert_eq!(config.database_type, db_type);
  }

  #[test]
  fn test_consumer_config_with_table_name(
    table_name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_table_name(table_name.clone());
    prop_assert_eq!(config.table_name, table_name);
  }

  #[test]
  fn test_consumer_config_with_insert_query(
    query in prop::string::string_regex(".*").unwrap()
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_insert_query(query.clone());
    prop_assert_eq!(config.insert_query, Some(query));
  }

  #[test]
  fn test_consumer_config_with_batch_size(
    batch_size in 1usize..10000usize
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_batch_size(batch_size);
    prop_assert_eq!(config.batch_size, batch_size);
  }

  #[test]
  fn test_consumer_config_with_batch_timeout(
    seconds in 0u64..3600u64
  ) {
    let timeout = Duration::from_secs(seconds);
    let config = DatabaseConsumerConfig::default()
      .with_batch_timeout(timeout);
    prop_assert_eq!(config.batch_timeout, timeout);
  }

  #[test]
  fn test_consumer_config_with_column_mapping(
    mapping in prop::collection::hash_map(
      column_name_strategy(),
      column_name_strategy(),
      0..20
    )
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_column_mapping(mapping.clone());
    prop_assert_eq!(config.column_mapping, Some(mapping));
  }

  #[test]
  fn test_consumer_config_with_transactions(
    use_transactions in any::<bool>()
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_transactions(use_transactions);
    prop_assert_eq!(config.use_transactions, use_transactions);
  }

  #[test]
  fn test_consumer_config_with_transaction_size(
    size in prop::option::of(1usize..10000usize)
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_transaction_size(size);
    prop_assert_eq!(config.transaction_size, size);
  }

  #[test]
  fn test_consumer_config_with_max_connections(
    max_conns in 1u32..1000u32
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_max_connections(max_conns);
    prop_assert_eq!(config.max_connections, max_conns);
  }

  #[test]
  fn test_consumer_config_with_ssl(
    enable_ssl in any::<bool>()
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_ssl(enable_ssl);
    prop_assert_eq!(config.enable_ssl, enable_ssl);
  }

  #[test]
  fn test_consumer_config_builder_chaining(
    url in prop::string::string_regex(".*").unwrap(),
    db_type in database_type_strategy(),
    table_name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    batch_size in 1usize..10000usize,
    seconds in 0u64..3600u64,
    use_transactions in any::<bool>(),
    enable_ssl in any::<bool>(),
  ) {
    let timeout = Duration::from_secs(seconds);
    let config = DatabaseConsumerConfig::default()
      .with_connection_url(url.clone())
      .with_database_type(db_type)
      .with_table_name(table_name.clone())
      .with_batch_size(batch_size)
      .with_batch_timeout(timeout)
      .with_transactions(use_transactions)
      .with_ssl(enable_ssl);

    prop_assert_eq!(config.connection_url, url);
    prop_assert_eq!(config.table_name, table_name);
    prop_assert_eq!(config.batch_size, batch_size);
    prop_assert_eq!(config.batch_timeout, timeout);
    prop_assert_eq!(config.use_transactions, use_transactions);
    prop_assert_eq!(config.enable_ssl, enable_ssl);
  }

  #[test]
  fn test_consumer_config_clone(
    url in prop::string::string_regex(".*").unwrap(),
    table_name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
  ) {
    let config = DatabaseConsumerConfig::default()
      .with_connection_url(url.clone())
      .with_table_name(table_name.clone());
    let cloned = config.clone();

    prop_assert_eq!(config.connection_url, cloned.connection_url);
    prop_assert_eq!(config.table_name, cloned.table_name);
    prop_assert_eq!(config.database_type, cloned.database_type);
  }

  #[test]
  fn test_consumer_config_default_values(_ in any::<u8>()) {
    let config = DatabaseConsumerConfig::default();
    prop_assert_eq!(config.database_type, DatabaseType::Postgres);
    prop_assert_eq!(config.batch_size, 100);
    prop_assert_eq!(config.batch_timeout, Duration::from_secs(5));
    prop_assert!(config.use_transactions);
    prop_assert_eq!(config.transaction_size, None);
    prop_assert_eq!(config.connection_url, "");
    prop_assert_eq!(config.table_name, "");
    prop_assert_eq!(config.insert_query, None);
    prop_assert_eq!(config.column_mapping, None);
    prop_assert_eq!(config.max_connections, 10);
    prop_assert_eq!(config.min_connections, 2);
    prop_assert!(!config.enable_ssl);
  }

  // Cross-type property tests
  #[test]
  fn test_database_row_get_all_columns(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());
    let columns = row.columns();

    // Every column should be retrievable via get()
    for col in &columns {
      prop_assert!(row.get(col).is_some());
    }
  }

  #[test]
  fn test_database_row_get_matches_fields(
    fields in database_row_fields_strategy()
  ) {
    let row = DatabaseRow::new(fields.clone());

    // get() should return the same value as fields.get()
    for (key, value) in &fields {
      prop_assert_eq!(row.get(key), Some(value));
    }
  }

  #[test]
  fn test_producer_config_parameter_ordering(
    params in prop::collection::vec(json_value_strategy(), 0..20)
  ) {
    let mut config = DatabaseProducerConfig::default();
    for param in &params {
      config = config.with_parameter(param.clone());
    }

    prop_assert_eq!(config.parameters.len(), params.len());
    for (i, param) in params.iter().enumerate() {
      prop_assert_eq!(config.parameters[i], *param);
    }
  }
}
