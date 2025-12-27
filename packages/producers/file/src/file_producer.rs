use streamweave_core::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that reads items from a file.
///
/// This producer reads lines from a file and emits each line as a string item.
pub struct FileProducer {
  /// The path to the file to read from.
  pub path: String,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl FileProducer {
  /// Creates a new `FileProducer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file to read from.
  pub fn new(path: String) -> Self {
    Self {
      path,
      config: streamweave_core::ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
