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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
  /// The name of this consumer component.
  pub name: String,
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
      name: String::new(),
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

  /// Sets the name for this consumer configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }
}
