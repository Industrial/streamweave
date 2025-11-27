use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;

/// A consumer that collects string items into a single `String`.
///
/// This consumer concatenates all string items from the stream into an internal buffer,
/// producing a single combined string.
pub struct StringConsumer {
  /// The internal buffer where strings are collected.
  pub buffer: String,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<String>,
}

impl Default for StringConsumer {
  fn default() -> Self {
    Self::new()
  }
}

impl StringConsumer {
  /// Creates a new `StringConsumer` with an empty buffer.
  pub fn new() -> Self {
    Self {
      buffer: String::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Creates a new `StringConsumer` with a pre-allocated buffer capacity.
  ///
  /// # Arguments
  ///
  /// * `capacity` - The initial capacity of the internal buffer.
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      buffer: String::with_capacity(capacity),
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

  /// Consumes the consumer and returns the collected `String`.
  ///
  /// # Returns
  ///
  /// The `String` containing all concatenated items.
  pub fn into_string(self) -> String {
    self.buffer
  }
}
