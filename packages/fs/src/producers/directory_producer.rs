use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that reads directory entries from a directory path.
///
/// Emits paths to files and subdirectories found in the specified directory.
pub struct DirectoryProducer {
  /// The directory path to read from
  pub path: String,
  /// Whether to read recursively
  pub recursive: bool,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl DirectoryProducer {
  /// Creates a new `DirectoryProducer` with the given directory path.
  ///
  /// # Arguments
  ///
  /// * `path` - The directory path to read from
  pub fn new(path: String) -> Self {
    Self {
      path,
      recursive: false,
      config: ProducerConfig::default(),
    }
  }

  /// Sets whether to read directories recursively.
  pub fn recursive(mut self, recursive: bool) -> Self {
    self.recursive = recursive;
    self
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
