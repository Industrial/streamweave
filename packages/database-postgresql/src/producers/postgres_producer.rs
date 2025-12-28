use streamweave::ProducerConfig;
use streamweave_database::{DatabaseProducerConfig, DatabaseRow};
use streamweave_error::ErrorStrategy;

/// A producer that executes PostgreSQL queries and streams the results.
///
/// This producer uses connection pooling and provides cursor-based iteration
/// for large result sets to keep memory usage bounded.
pub struct PostgresProducer {
  /// Producer configuration.
  pub config: ProducerConfig<DatabaseRow>,
  /// Database-specific configuration.
  pub db_config: DatabaseProducerConfig,
  /// Connection pool (initialized lazily).
  pub(crate) pool: Option<sqlx::PgPool>,
}

impl PostgresProducer {
  /// Creates a new PostgreSQL producer with the given configuration.
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

impl Clone for PostgresProducer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      db_config: self.db_config.clone(),
      pool: None, // Pool cannot be cloned, will be re-created
    }
  }
}
