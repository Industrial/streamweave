use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use tokio::fs::File;

/// A consumer that writes string items to a file.
///
/// This consumer writes each item from the stream to the specified file path.
/// The file is opened lazily when the first item is consumed.
pub struct FileConsumer {
  /// The file handle, opened when the first item is consumed.
  pub file: Option<File>,
  /// The path to the file where items will be written.
  pub path: String,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<String>,
}

impl FileConsumer {
  /// Creates a new `FileConsumer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file where items will be written.
  pub fn new(path: String) -> Self {
    Self {
      file: None,
      path,
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
