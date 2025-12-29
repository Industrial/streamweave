use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that writes items to a temporary file.
///
/// The temporary file is managed externally and will be cleaned up when dropped.
pub struct TempFileConsumer {
  /// The temporary file path
  pub path: std::path::PathBuf,
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl TempFileConsumer {
  /// Creates a new `TempFileConsumer` from a temporary file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the temporary file
  pub fn new(path: std::path::PathBuf) -> Self {
    Self {
      path,
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
