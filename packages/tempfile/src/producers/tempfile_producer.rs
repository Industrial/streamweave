use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that reads lines from a temporary file.
///
/// The temporary file is created and managed by the `tempfile` crate.
pub struct TempFileProducer {
  /// The temporary file path
  pub path: std::path::PathBuf,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl TempFileProducer {
  /// Creates a new `TempFileProducer` from a temporary file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the temporary file
  pub fn new(path: std::path::PathBuf) -> Self {
    Self {
      path,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
