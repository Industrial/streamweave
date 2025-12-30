use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Database type supported by the producer and consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DatabaseType {
  /// PostgreSQL database
  #[default]
  Postgres,
  /// MySQL/MariaDB database
  Mysql,
  /// SQLite database (including in-memory)
  Sqlite,
}

/// A row result from a database query.
///
/// This type represents a single row from a database query result,
/// with column names as keys and values as JSON values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseRow {
  /// Column names and their values.
  pub fields: HashMap<String, serde_json::Value>,
}

impl DatabaseRow {
  /// Creates a new database row from a HashMap of fields.
  #[must_use]
  pub fn new(fields: HashMap<String, serde_json::Value>) -> Self {
    Self { fields }
  }

  /// Gets a value by column name.
  #[must_use]
  pub fn get(&self, column: &str) -> Option<&serde_json::Value> {
    self.fields.get(column)
  }

  /// Gets all column names.
  #[must_use]
  pub fn columns(&self) -> Vec<String> {
    self.fields.keys().cloned().collect()
  }
}

/// Configuration for database query producer behavior.
#[derive(Debug, Clone)]
pub struct DatabaseProducerConfig {
  /// Database connection URL (e.g., "postgresql://user:pass@localhost/dbname").
  pub connection_url: String,
  /// Type of database (PostgreSQL, MySQL, or SQLite).
  pub database_type: DatabaseType,
  /// SQL query to execute.
  pub query: String,
  /// Query parameters (for parameterized queries).
  pub parameters: Vec<serde_json::Value>,
  /// Maximum number of connections in the pool.
  pub max_connections: u32,
  /// Minimum number of idle connections in the pool.
  pub min_connections: u32,
  /// Connection timeout.
  pub connect_timeout: Duration,
  /// Maximum idle time before closing a connection.
  pub idle_timeout: Option<Duration>,
  /// Maximum lifetime of a connection.
  pub max_lifetime: Option<Duration>,
  /// Fetch size for cursor-based iteration (number of rows per fetch).
  pub fetch_size: usize,
  /// Whether to enable SSL/TLS for the connection.
  pub enable_ssl: bool,
}

impl Default for DatabaseProducerConfig {
  fn default() -> Self {
    Self {
      connection_url: String::new(),
      database_type: DatabaseType::Postgres,
      query: String::new(),
      parameters: Vec::new(),
      max_connections: 10,
      min_connections: 2,
      connect_timeout: Duration::from_secs(30),
      idle_timeout: Some(Duration::from_secs(600)),
      max_lifetime: Some(Duration::from_secs(1800)),
      fetch_size: 1000,
      enable_ssl: false,
    }
  }
}

impl DatabaseProducerConfig {
  /// Sets the database connection URL.
  #[must_use]
  pub fn with_connection_url(mut self, url: impl Into<String>) -> Self {
    self.connection_url = url.into();
    self
  }

  /// Sets the database type.
  #[must_use]
  pub fn with_database_type(mut self, db_type: DatabaseType) -> Self {
    self.database_type = db_type;
    self
  }

  /// Sets the SQL query to execute.
  #[must_use]
  pub fn with_query(mut self, query: impl Into<String>) -> Self {
    self.query = query.into();
    self
  }

  /// Adds a parameter to the query.
  #[must_use]
  pub fn with_parameter(mut self, param: serde_json::Value) -> Self {
    self.parameters.push(param);
    self
  }

  /// Sets multiple parameters at once.
  #[must_use]
  pub fn with_parameters(mut self, params: Vec<serde_json::Value>) -> Self {
    self.parameters = params;
    self
  }

  /// Sets the maximum number of connections in the pool.
  #[must_use]
  pub fn with_max_connections(mut self, max: u32) -> Self {
    self.max_connections = max;
    self
  }

  /// Sets the fetch size for cursor-based iteration.
  #[must_use]
  pub fn with_fetch_size(mut self, size: usize) -> Self {
    self.fetch_size = size;
    self
  }

  /// Sets whether to enable SSL/TLS.
  #[must_use]
  pub fn with_ssl(mut self, enable: bool) -> Self {
    self.enable_ssl = enable;
    self
  }
}

/// Configuration for database consumer behavior.
#[derive(Debug, Clone)]
pub struct DatabaseConsumerConfig {
  /// Database connection URL (e.g., "postgresql://user:pass@localhost/dbname").
  pub connection_url: String,
  /// Type of database (PostgreSQL, MySQL, or SQLite).
  pub database_type: DatabaseType,
  /// Target table name for inserts.
  pub table_name: String,
  /// Custom INSERT query. If None, will be auto-generated from table_name and row columns.
  pub insert_query: Option<String>,
  /// Number of rows to batch before inserting.
  pub batch_size: usize,
  /// Maximum time to wait before flushing a batch (even if not full).
  pub batch_timeout: Duration,
  /// Column mapping: maps DatabaseRow field names to table column names.
  /// If None, assumes field names match column names exactly.
  pub column_mapping: Option<HashMap<String, String>>,
  /// Whether to use transactions for batch inserts.
  pub use_transactions: bool,
  /// Commit transaction every N rows. If None, commits at end of stream.
  pub transaction_size: Option<usize>,
  /// Maximum number of connections in the pool.
  pub max_connections: u32,
  /// Minimum number of idle connections in the pool.
  pub min_connections: u32,
  /// Connection timeout.
  pub connect_timeout: Duration,
  /// Maximum idle time before closing a connection.
  pub idle_timeout: Option<Duration>,
  /// Maximum lifetime of a connection.
  pub max_lifetime: Option<Duration>,
  /// Whether to enable SSL/TLS for the connection.
  pub enable_ssl: bool,
}

impl Default for DatabaseConsumerConfig {
  fn default() -> Self {
    Self {
      connection_url: String::new(),
      database_type: DatabaseType::Postgres,
      table_name: String::new(),
      insert_query: None,
      batch_size: 100,
      batch_timeout: Duration::from_secs(5),
      column_mapping: None,
      use_transactions: true,
      transaction_size: None,
      max_connections: 10,
      min_connections: 2,
      connect_timeout: Duration::from_secs(30),
      idle_timeout: Some(Duration::from_secs(600)),
      max_lifetime: Some(Duration::from_secs(1800)),
      enable_ssl: false,
    }
  }
}

impl DatabaseConsumerConfig {
  /// Sets the database connection URL.
  #[must_use]
  pub fn with_connection_url(mut self, url: impl Into<String>) -> Self {
    self.connection_url = url.into();
    self
  }

  /// Sets the database type.
  #[must_use]
  pub fn with_database_type(mut self, db_type: DatabaseType) -> Self {
    self.database_type = db_type;
    self
  }

  /// Sets the target table name.
  #[must_use]
  pub fn with_table_name(mut self, table: impl Into<String>) -> Self {
    self.table_name = table.into();
    self
  }

  /// Sets a custom INSERT query.
  #[must_use]
  pub fn with_insert_query(mut self, query: impl Into<String>) -> Self {
    self.insert_query = Some(query.into());
    self
  }

  /// Sets the batch size.
  #[must_use]
  pub fn with_batch_size(mut self, size: usize) -> Self {
    self.batch_size = size;
    self
  }

  /// Sets the batch timeout.
  #[must_use]
  pub fn with_batch_timeout(mut self, timeout: Duration) -> Self {
    self.batch_timeout = timeout;
    self
  }

  /// Sets column mapping.
  #[must_use]
  pub fn with_column_mapping(mut self, mapping: HashMap<String, String>) -> Self {
    self.column_mapping = Some(mapping);
    self
  }

  /// Sets whether to use transactions.
  #[must_use]
  pub fn with_transactions(mut self, use_transactions: bool) -> Self {
    self.use_transactions = use_transactions;
    self
  }

  /// Sets transaction commit size.
  #[must_use]
  pub fn with_transaction_size(mut self, size: Option<usize>) -> Self {
    self.transaction_size = size;
    self
  }

  /// Sets the maximum number of connections in the pool.
  #[must_use]
  pub fn with_max_connections(mut self, max: u32) -> Self {
    self.max_connections = max;
    self
  }

  /// Sets whether to enable SSL/TLS.
  #[must_use]
  pub fn with_ssl(mut self, enable: bool) -> Self {
    self.enable_ssl = enable;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // DatabaseType tests
  #[test]
  fn test_database_type_default() {
    assert_eq!(DatabaseType::default(), DatabaseType::Postgres);
  }

  #[test]
  fn test_database_type_variants() {
    assert_eq!(DatabaseType::Postgres, DatabaseType::Postgres);
    assert_eq!(DatabaseType::Mysql, DatabaseType::Mysql);
    assert_eq!(DatabaseType::Sqlite, DatabaseType::Sqlite);
    assert_ne!(DatabaseType::Postgres, DatabaseType::Mysql);
    assert_ne!(DatabaseType::Postgres, DatabaseType::Sqlite);
    assert_ne!(DatabaseType::Mysql, DatabaseType::Sqlite);
  }

  #[test]
  fn test_database_type_serialize_deserialize() {
    let postgres = DatabaseType::Postgres;
    let mysql = DatabaseType::Mysql;
    let sqlite = DatabaseType::Sqlite;

    let postgres_json = serde_json::to_string(&postgres).unwrap();
    let mysql_json = serde_json::to_string(&mysql).unwrap();
    let sqlite_json = serde_json::to_string(&sqlite).unwrap();

    assert_eq!(postgres_json, "\"Postgres\"");
    assert_eq!(mysql_json, "\"Mysql\"");
    assert_eq!(sqlite_json, "\"Sqlite\"");

    let postgres_deser: DatabaseType = serde_json::from_str(&postgres_json).unwrap();
    let mysql_deser: DatabaseType = serde_json::from_str(&mysql_json).unwrap();
    let sqlite_deser: DatabaseType = serde_json::from_str(&sqlite_json).unwrap();

    assert_eq!(postgres_deser, DatabaseType::Postgres);
    assert_eq!(mysql_deser, DatabaseType::Mysql);
    assert_eq!(sqlite_deser, DatabaseType::Sqlite);
  }

  // DatabaseRow tests
  #[test]
  fn test_database_row_new() {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
    fields.insert(
      "name".to_string(),
      serde_json::Value::String("Alice".to_string()),
    );

    let row = DatabaseRow::new(fields);
    assert_eq!(row.fields.len(), 2);
  }

  #[test]
  fn test_database_row_get() {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
    fields.insert(
      "name".to_string(),
      serde_json::Value::String("Alice".to_string()),
    );

    let row = DatabaseRow::new(fields);
    assert_eq!(row.get("id"), Some(&serde_json::Value::Number(1.into())));
    assert_eq!(
      row.get("name"),
      Some(&serde_json::Value::String("Alice".to_string()))
    );
    assert_eq!(row.get("nonexistent"), None);
  }

  #[test]
  fn test_database_row_get_all_types() {
    let mut fields = HashMap::new();
    fields.insert("bool".to_string(), serde_json::Value::Bool(true));
    fields.insert("number".to_string(), serde_json::Value::Number(42.into()));
    fields.insert(
      "string".to_string(),
      serde_json::Value::String("test".to_string()),
    );
    fields.insert("null".to_string(), serde_json::Value::Null);
    fields.insert(
      "array".to_string(),
      serde_json::Value::Array(vec![serde_json::Value::Number(1.into())]),
    );
    fields.insert("object".to_string(), serde_json::json!({"key": "value"}));

    let row = DatabaseRow::new(fields);
    assert_eq!(row.get("bool"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(
      row.get("number"),
      Some(&serde_json::Value::Number(42.into()))
    );
    assert_eq!(
      row.get("string"),
      Some(&serde_json::Value::String("test".to_string()))
    );
    assert_eq!(row.get("null"), Some(&serde_json::Value::Null));
    assert!(row.get("array").is_some());
    assert!(row.get("object").is_some());
  }

  #[test]
  fn test_database_row_columns() {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
    fields.insert(
      "name".to_string(),
      serde_json::Value::String("Alice".to_string()),
    );
    fields.insert(
      "email".to_string(),
      serde_json::Value::String("alice@example.com".to_string()),
    );

    let row = DatabaseRow::new(fields);
    let columns = row.columns();
    assert_eq!(columns.len(), 3);
    assert!(columns.contains(&"id".to_string()));
    assert!(columns.contains(&"name".to_string()));
    assert!(columns.contains(&"email".to_string()));
  }

  #[test]
  fn test_database_row_empty() {
    let fields = HashMap::new();
    let row = DatabaseRow::new(fields);
    assert_eq!(row.fields.len(), 0);
    assert_eq!(row.columns().len(), 0);
    assert_eq!(row.get("any"), None);
  }

  #[test]
  fn test_database_row_clone() {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
    let row = DatabaseRow::new(fields);
    let cloned = row.clone();
    assert_eq!(row.get("id"), cloned.get("id"));
  }

  #[test]
  fn test_database_row_serialize_deserialize() {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
    fields.insert(
      "name".to_string(),
      serde_json::Value::String("Alice".to_string()),
    );
    let row = DatabaseRow::new(fields);

    let json = serde_json::to_string(&row).unwrap();
    let deserialized: DatabaseRow = serde_json::from_str(&json).unwrap();

    assert_eq!(row.get("id"), deserialized.get("id"));
    assert_eq!(row.get("name"), deserialized.get("name"));
    assert_eq!(row.columns().len(), deserialized.columns().len());
  }

  // DatabaseProducerConfig tests
  #[test]
  fn test_database_producer_config_default() {
    let config = DatabaseProducerConfig::default();
    assert_eq!(config.database_type, DatabaseType::Postgres);
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.min_connections, 2);
    assert_eq!(config.fetch_size, 1000);
    assert!(!config.enable_ssl);
    assert_eq!(config.connection_url, "");
    assert_eq!(config.query, "");
    assert_eq!(config.parameters.len(), 0);
    assert_eq!(config.connect_timeout, Duration::from_secs(30));
    assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
    assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
  }

  #[test]
  fn test_database_producer_config_with_connection_url() {
    let config =
      DatabaseProducerConfig::default().with_connection_url("postgresql://user:pass@localhost/db");
    assert_eq!(config.connection_url, "postgresql://user:pass@localhost/db");
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
      .with_parameter(serde_json::Value::Number(1.into()))
      .with_parameter(serde_json::Value::String("test".to_string()))
      .with_parameter(serde_json::Value::Bool(true));

    assert_eq!(config.parameters.len(), 3);
    assert_eq!(config.parameters[0], serde_json::Value::Number(1.into()));
    assert_eq!(
      config.parameters[1],
      serde_json::Value::String("test".to_string())
    );
    assert_eq!(config.parameters[2], serde_json::Value::Bool(true));
  }

  #[test]
  fn test_database_producer_config_with_parameters() {
    let params = vec![
      serde_json::Value::Number(1.into()),
      serde_json::Value::String("test".to_string()),
    ];
    let config = DatabaseProducerConfig::default().with_parameters(params.clone());
    assert_eq!(config.parameters, params);
  }

  #[test]
  fn test_database_producer_config_with_max_connections() {
    let config = DatabaseProducerConfig::default().with_max_connections(50);
    assert_eq!(config.max_connections, 50);
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
  fn test_database_producer_config_chaining() {
    let config = DatabaseProducerConfig::default()
      .with_connection_url("postgresql://localhost/db")
      .with_database_type(DatabaseType::Postgres)
      .with_query("SELECT * FROM users")
      .with_parameter(serde_json::Value::Number(1.into()))
      .with_max_connections(20)
      .with_fetch_size(500)
      .with_ssl(true);

    assert_eq!(config.connection_url, "postgresql://localhost/db");
    assert_eq!(config.database_type, DatabaseType::Postgres);
    assert_eq!(config.query, "SELECT * FROM users");
    assert_eq!(config.parameters.len(), 1);
    assert_eq!(config.max_connections, 20);
    assert_eq!(config.fetch_size, 500);
    assert!(config.enable_ssl);
  }

  #[test]
  fn test_database_producer_config_clone() {
    let config = DatabaseProducerConfig::default()
      .with_connection_url("postgresql://localhost/db")
      .with_query("SELECT * FROM users");
    let cloned = config.clone();
    assert_eq!(config.connection_url, cloned.connection_url);
    assert_eq!(config.query, cloned.query);
  }

  // DatabaseConsumerConfig tests
  #[test]
  fn test_database_consumer_config_default() {
    let config = DatabaseConsumerConfig::default();
    assert_eq!(config.database_type, DatabaseType::Postgres);
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.batch_timeout, Duration::from_secs(5));
    assert!(config.use_transactions);
    assert_eq!(config.transaction_size, None);
    assert_eq!(config.connection_url, "");
    assert_eq!(config.table_name, "");
    assert_eq!(config.insert_query, None);
    assert_eq!(config.column_mapping, None);
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.min_connections, 2);
    assert_eq!(config.connect_timeout, Duration::from_secs(30));
    assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
    assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
    assert!(!config.enable_ssl);
  }

  #[test]
  fn test_database_consumer_config_with_connection_url() {
    let config =
      DatabaseConsumerConfig::default().with_connection_url("postgresql://user:pass@localhost/db");
    assert_eq!(config.connection_url, "postgresql://user:pass@localhost/db");
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
    let config = DatabaseConsumerConfig::default().with_batch_size(1000);
    assert_eq!(config.batch_size, 1000);
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
    mapping.insert("user_id".to_string(), "id".to_string());
    mapping.insert("user_name".to_string(), "name".to_string());

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
    let config = DatabaseConsumerConfig::default().with_transaction_size(Some(500));
    assert_eq!(config.transaction_size, Some(500));

    let config = DatabaseConsumerConfig::default().with_transaction_size(None);
    assert_eq!(config.transaction_size, None);
  }

  #[test]
  fn test_database_consumer_config_with_max_connections() {
    let config = DatabaseConsumerConfig::default().with_max_connections(50);
    assert_eq!(config.max_connections, 50);
  }

  #[test]
  fn test_database_consumer_config_with_ssl() {
    let config = DatabaseConsumerConfig::default().with_ssl(true);
    assert!(config.enable_ssl);

    let config = DatabaseConsumerConfig::default().with_ssl(false);
    assert!(!config.enable_ssl);
  }

  #[test]
  fn test_database_consumer_config_chaining() {
    let mut mapping = HashMap::new();
    mapping.insert("user_id".to_string(), "id".to_string());

    let config = DatabaseConsumerConfig::default()
      .with_connection_url("postgresql://localhost/db")
      .with_database_type(DatabaseType::Postgres)
      .with_table_name("users")
      .with_batch_size(1000)
      .with_batch_timeout(Duration::from_secs(10))
      .with_column_mapping(mapping.clone())
      .with_transactions(true)
      .with_transaction_size(Some(500))
      .with_max_connections(20)
      .with_ssl(true);

    assert_eq!(config.connection_url, "postgresql://localhost/db");
    assert_eq!(config.database_type, DatabaseType::Postgres);
    assert_eq!(config.table_name, "users");
    assert_eq!(config.batch_size, 1000);
    assert_eq!(config.batch_timeout, Duration::from_secs(10));
    assert_eq!(config.column_mapping, Some(mapping));
    assert!(config.use_transactions);
    assert_eq!(config.transaction_size, Some(500));
    assert_eq!(config.max_connections, 20);
    assert!(config.enable_ssl);
  }

  #[test]
  fn test_database_consumer_config_clone() {
    let config = DatabaseConsumerConfig::default()
      .with_connection_url("postgresql://localhost/db")
      .with_table_name("users");
    let cloned = config.clone();
    assert_eq!(config.connection_url, cloned.connection_url);
    assert_eq!(config.table_name, cloned.table_name);
  }
}
