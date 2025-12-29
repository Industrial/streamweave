use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that creates directories from stream items (path strings).
pub struct DirectoryConsumer {
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl DirectoryConsumer {
  /// Creates a new `DirectoryConsumer`.
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl Default for DirectoryConsumer {
  fn default() -> Self {
    Self::new()
  }
}
