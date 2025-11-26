use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Database type supported by the producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
  /// PostgreSQL database
  Postgres,
  /// MySQL/MariaDB database
  Mysql,
  /// SQLite database (including in-memory)
  Sqlite,
}

impl Default for DatabaseType {
  fn default() -> Self {
    Self::Postgres
  }
}

/// Configuration for database query producer behavior.
#[derive(Debug, Clone)]
pub struct DatabaseProducerConfig {
  /// Database connection URL (e.g., "postgresql://user:pass@localhost/dbname").
  pub connection_url: String,
  /// Type of database (PostgreSQL or MySQL).
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

/// A producer that executes database queries and streams the results.
///
/// This producer supports both PostgreSQL and MySQL databases, uses
/// connection pooling, and provides cursor-based iteration for large
/// result sets to keep memory usage bounded.
///
/// # Example
///
/// ```ignore
/// use streamweave::producers::database::{DatabaseProducer, DatabaseProducerConfig, DatabaseType};
///
/// let producer = DatabaseProducer::new(
///     DatabaseProducerConfig::default()
///         .with_connection_url("postgresql://user:pass@localhost/dbname")
///         .with_database_type(DatabaseType::Postgres)
///         .with_query("SELECT id, name, email FROM users WHERE active = $1")
///         .with_parameter(serde_json::Value::Bool(true))
///         .with_fetch_size(100)
/// );
/// ```
pub struct DatabaseProducer {
  /// Producer configuration.
  pub config: ProducerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseProducerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<DatabasePool>,
}

/// Internal enum to hold different database connection pools.
#[derive(Debug)]
pub(crate) enum DatabasePool {
  #[cfg(feature = "sqlx")]
  Postgres(sqlx::PgPool),
  #[cfg(feature = "sqlx")]
  Mysql(sqlx::MySqlPool),
  #[cfg(feature = "sqlx")]
  Sqlite(sqlx::SqlitePool),
}

impl DatabaseProducer {
  /// Creates a new database producer with the given configuration.
  #[must_use]
  pub fn new(db_config: DatabaseProducerConfig) -> Self {
    Self {
      config: ProducerConfig::default(),
      db_config,
      pool: None,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<DatabaseRow>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the database producer configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseProducerConfig {
    &self.db_config
  }
}

impl Clone for DatabaseProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_database_producer_config_default() {
    let config = DatabaseProducerConfig::default();
    assert_eq!(config.database_type, DatabaseType::Postgres);
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.fetch_size, 1000);
    assert!(!config.enable_ssl);
  }

  #[test]
  fn test_database_producer_config_builder() {
    let config = DatabaseProducerConfig::default()
      .with_connection_url("postgresql://user:pass@localhost/db")
      .with_database_type(DatabaseType::Mysql)
      .with_query("SELECT * FROM users")
      .with_parameter(serde_json::Value::Number(1.into()))
      .with_max_connections(20)
      .with_fetch_size(500)
      .with_ssl(true);

    assert_eq!(config.connection_url, "postgresql://user:pass@localhost/db");
    assert_eq!(config.database_type, DatabaseType::Mysql);
    assert_eq!(config.query, "SELECT * FROM users");
    assert_eq!(config.parameters.len(), 1);
    assert_eq!(config.max_connections, 20);
    assert_eq!(config.fetch_size, 500);
    assert!(config.enable_ssl);
  }

  #[test]
  fn test_database_producer_config_sqlite() {
    let config = DatabaseProducerConfig::default()
      .with_connection_url("sqlite::memory:")
      .with_database_type(DatabaseType::Sqlite)
      .with_query("SELECT * FROM users WHERE id = ?");

    assert_eq!(config.connection_url, "sqlite::memory:");
    assert_eq!(config.database_type, DatabaseType::Sqlite);
  }

  #[test]
  fn test_database_row() {
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
    assert_eq!(row.columns().len(), 2);
  }
}
