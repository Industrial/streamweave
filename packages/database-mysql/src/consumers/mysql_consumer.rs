use std::time::Instant;
use streamweave::ConsumerConfig;
use streamweave_database::{DatabaseConsumerConfig, DatabaseRow};
use streamweave_error::ErrorStrategy;

/// A consumer that writes database rows to a MySQL table.
///
/// This consumer supports batch insertion for performance and transaction management.
pub struct MysqlConsumer {
  /// Consumer configuration.
  pub config: ConsumerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseConsumerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::MySqlPool>,
  /// Buffer for batch insertion.
  pub(crate) batch_buffer: Vec<DatabaseRow>,
  /// Timestamp of last batch flush.
  pub(crate) last_flush: Instant,
}

impl MysqlConsumer {
  /// Creates a new MySQL consumer with the given configuration.
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

impl Clone for MysqlConsumer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None,
      batch_buffer: Vec::new(),
      last_flush: Instant::now(),
    }
  }
}
