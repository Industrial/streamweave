use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

/// A producer that generates items from a string by chunking it.
///
/// This producer splits a string into chunks of a specified size and emits
/// each chunk as a separate item in the stream.
pub struct StringProducer {
  /// The string data to produce.
  pub data: String,
  /// The size of each chunk to split the string into.
  pub chunk_size: usize,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl StringProducer {
  /// Creates a new `StringProducer` with the given string and chunk size.
  ///
  /// # Arguments
  ///
  /// * `data` - The string to produce.
  /// * `chunk_size` - The size of each chunk to split the string into.
  pub fn new(data: String, chunk_size: usize) -> Self {
    Self {
      data,
      chunk_size,
      config: crate::producer::ProducerConfig::default(),
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
