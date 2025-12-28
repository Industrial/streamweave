use super::super::producers::database_producer::{
  DatabaseConsumerConfig, DatabasePool, DatabaseRow,
};
use std::time::Instant;
use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that writes database rows to a database table.
///
/// This consumer supports batch insertion for performance, transaction management,
/// and works with PostgreSQL, MySQL, and SQLite databases.
///
/// # Example
///
/// ```ignore
/// use streamweave::consumers::streamweave_database::{DatabaseConsumer, DatabaseConsumerConfig, DatabaseType};
///
/// let consumer = DatabaseConsumer::new(
///     DatabaseConsumerConfig::default()
///         .with_connection_url("postgresql://user:pass@localhost/dbname")
///         .with_database_type(DatabaseType::Postgres)
///         .with_table_name("users")
///         .with_batch_size(100)
/// );
/// ```
pub struct DatabaseConsumer {
  /// Consumer configuration.
  pub config: ConsumerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseConsumerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<DatabasePool>,
  /// Buffer for batch insertion.
  pub(crate) batch_buffer: Vec<DatabaseRow>,
  /// Timestamp of last batch flush.
  pub(crate) last_flush: Instant,
}

impl DatabaseConsumer {
  /// Creates a new database consumer with the given configuration.
  #[must_use]
  pub fn new(db_config: DatabaseConsumerConfig) -> Self {
    Self {
      config: ConsumerConfig::default(),
      db_config,
      pool: None,
      batch_buffer: Vec::new(),
      last_flush: Instant::now(),
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<DatabaseRow>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Returns the database consumer configuration.
  #[must_use]
  pub fn db_config(&self) -> &DatabaseConsumerConfig {
    &self.db_config
  }
}

impl Clone for DatabaseConsumer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
      batch_buffer: Vec::new(),
      last_flush: Instant::now(),
    }
  }
}
